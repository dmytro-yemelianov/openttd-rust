#!/usr/bin/env python3
"""Project-local pi launcher. Standard library only; credentials never reach pi."""
import argparse, fcntl, getpass, json, os, pathlib, secrets, subprocess, sys, threading, time
import urllib.request, urllib.error
from http.server import HTTPServer, BaseHTTPRequestHandler

NVIDIA = 'https://integrate.api.nvidia.com/v1/chat/completions'
ZAI = 'https://api.z.ai/api/paas/v4/chat/completions'
MODEL = 'nvidia/nemotron-3-super-120b-a12b'
PAID_MODEL = 'glm-5.3-flash'
RETRYABLE = {408, 429, 500, 502, 503, 504}
# Conservative per-call reservation: 1M input tokens at $0.15/M plus
# 8192 output tokens at $0.50/M, rounded up. No refunds, even on failure.
# Valid for pinned published rates; provider account prepaid balance is final guard.
RESERVE_MICRO = 160000
LIMIT_MICRO = 10000000
MAX_OUTPUT = 8192

class RouteError(Exception):
    pass

def save(path, value):
    tmp = path.with_suffix('.tmp')
    with open(tmp, 'w') as f:
        os.chmod(tmp, 0o600)
        json.dump(value, f, indent=2)
        f.flush(); os.fsync(f.fileno())
    os.replace(tmp, path)

def reserve(path):
    # Launcher holds an exclusive process lock; HTTPServer is sequential.
    data = json.loads(path.read_text()) if path.exists() else {'reserved_microusd': 0, 'calls': 0}
    used = data['reserved_microusd']
    if not isinstance(used, int) or used < 0 or used + RESERVE_MICRO > LIMIT_MICRO:
        raise RouteError('Paid fallback budget exhausted; NVIDIA remains free.')
    data.update(reserved_microusd=used + RESERVE_MICRO, calls=data['calls'] + 1)
    save(path, data)

def upstream(url, key, payload):
    req = urllib.request.Request(url, data=json.dumps(payload).encode(), headers={
        'Authorization': 'Bearer ' + key, 'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=90) as response:
        return json.load(response)

class Router:
    def __init__(self, runtime, paid=False):
        self.runtime, self.paid = runtime, paid
        self.last_request = 0
        self.cooldown_until = 0
    def complete(self, body):
        messages = body.get('messages')
        if not isinstance(messages, list) or not messages:
            raise RouteError('A nonempty messages array is required.')
        # Bound requests before either endpoint receives them. Agent compaction
        # handles normal context growth; reject instead of silently truncating code.
        if len(json.dumps(body).encode()) > 120000:
            raise RouteError('Context exceeds launcher limit. Compact the session or start a bounded task.')
        payload = {k: body[k] for k in ('messages','tools','tool_choice','temperature','top_p') if k in body}
        payload.update(model=MODEL, stream=False, max_tokens=min(int(body.get('max_tokens', MAX_OUTPUT)), MAX_OUTPUT))
        if payload['max_tokens'] <= 0:
            raise RouteError('max_tokens must be positive')
        # Disabling thinking avoids spending the output allowance on hidden reasoning.
        payload['chat_template_kwargs'] = {'enable_thinking': False}
        key = os.environ.get('NVIDIA_API_KEY')
        if not key:
            keyfile = self.runtime / 'nvidia-key'
            key = keyfile.read_text().strip() if keyfile.exists() else ''
        if not key:
            raise RouteError('NVIDIA_API_KEY missing. Run this launcher with --configure-nvidia.')
        last = 'NVIDIA temporarily unavailable'
        if time.time() >= self.cooldown_until:
            for attempt in range(3):
                time.sleep(max(0, 3 - (time.monotonic() - self.last_request)))
                self.last_request = time.monotonic()
                try:
                    result = upstream(NVIDIA, key, payload)
                    if result.get('error') or not result.get('choices'):
                        raise RouteError('NVIDIA returned an invalid completion')
                    print('NVIDIA completion received', file=sys.stderr)
                    return result
                except urllib.error.HTTPError as e:
                    if e.code not in RETRYABLE:
                        raise RouteError(f'NVIDIA HTTP {e.code}; fix credentials/request. No paid fallback.')
                    last = f'NVIDIA HTTP {e.code}'
                    retry = e.headers.get('Retry-After', '')
                    delay = max(5 * 2**attempt, float(retry) if retry.isdigit() else 0)
                    if delay > 30:
                        self.cooldown_until = time.time() + delay
                        break
                    if attempt < 2:
                        print(f'{last}; retrying in {delay:g}s', file=sys.stderr)
                        time.sleep(delay)
                except (TimeoutError, OSError) as e:
                    last = 'NVIDIA network timeout/unavailable'
                    if attempt < 2: time.sleep(5 * 2**attempt)
            self.cooldown_until = max(self.cooldown_until, time.time() + 60)
        if not self.paid:
            raise RouteError(last + '; paid fallback disabled. Session saved; retry later.')
        keyfile = self.runtime / 'zai-key'
        if not keyfile.exists():
            raise RouteError('Paid fallback has no key. Run --configure-paid first.')
        reserve(self.runtime / 'paid-budget.json')
        payload.pop('chat_template_kwargs', None)
        payload.update(model=PAID_MODEL, thinking={'type': 'disabled'})
        print('Using paid Z.ai fallback; $0.16 reserved against $10 lifetime launcher budget.', file=sys.stderr)
        result = upstream(ZAI, keyfile.read_text().strip(), payload)
        if result.get('error') or not result.get('choices'):
            raise RouteError('Paid endpoint returned invalid completion; reservation retained.')
        return result

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_POST(self):
        if self.headers.get('Authorization') != 'Bearer ' + self.server.secret:
            self.send_error(401); return
        if self.path != '/v1/chat/completions':
            self.send_error(404); return
        try:
            size = int(self.headers.get('Content-Length', 0))
            if size <= 0 or size > 120000: raise RouteError('Request exceeds 120KB limit')
            body = json.loads(self.rfile.read(size))
            result = self.server.router.complete(body)
            # Buffer upstream completely before exposing a tool call to pi. This
            # permits safe inference failover without replaying executed tools.
            if body.get('stream'):
                choice = result['choices'][0]
                delta = choice['message']
                chunk = {'id': result.get('id','route'), 'object':'chat.completion.chunk',
                         'created':int(time.time()), 'model':body.get('model',MODEL),
                         'choices':[{'index':0,'delta':delta,'finish_reason':None}]}
                if delta.get('tool_calls'):
                    for i, call in enumerate(delta['tool_calls']): call['index'] = i
                end = dict(chunk, choices=[{'index':0,'delta':{},'finish_reason':choice.get('finish_reason','stop')}], usage=result.get('usage',{}))
                raw = ('data: '+json.dumps(chunk)+'\n\ndata: '+json.dumps(end)+'\n\ndata: [DONE]\n\n').encode()
                content_type = 'text/event-stream'
            else:
                raw = json.dumps(result).encode(); content_type='application/json'
            self.send_response(200); self.send_header('Content-Type', content_type)
        except Exception as e:
            # Do not echo upstream error bodies, request content or credentials.
            message = str(e) if isinstance(e, RouteError) else 'Request failed; check connectivity and provider account.'
            print(message, file=sys.stderr)
            raw=json.dumps({'error':{'message':message,'type':'router_error'}}).encode()
            self.send_response(400); self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(raw))); self.end_headers()
        try: self.wfile.write(raw)
        except (BrokenPipeError, ConnectionResetError): pass

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--configure-nvidia',action='store_true')
    parser.add_argument('--configure-paid',action='store_true')
    parser.add_argument('--paid',action='store_true',help='Explicitly enable prepared paid fallback')
    parser.add_argument('--status',action='store_true')
    parser.add_argument('--probe',action='store_true',help='Tiny NVIDIA-only API readiness request')
    args, pi_args=parser.parse_known_args()
    root=pathlib.Path(__file__).resolve().parents[1]
    runtime=root/'.git'/'pi-nvidia-first'
    runtime.mkdir(parents=True,exist_ok=True,mode=0o700)
    lock=open(runtime/'launcher.lock','a')
    try: fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError: sys.exit('Another NVIDIA-first launcher is running. Use one worker at a time.')
    if args.configure_nvidia or args.configure_paid:
        path=runtime/('zai-key' if args.configure_paid else 'nvidia-key')
        value=getpass.getpass('Z.ai pay-as-you-go API key: ' if args.configure_paid else 'NVIDIA API key: ').strip()
        if not value: sys.exit('No key entered; nothing changed.')
        fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
        with os.fdopen(fd,'w') as f: f.write(value)
        print('Key stored privately. Paid fallback requires --paid; configuring a key does not enable it.')
        return
    if args.status:
        print(json.dumps({'primary':MODEL,'nvidia_key_available':bool(os.environ.get('NVIDIA_API_KEY') or (runtime/'nvidia-key').exists()),'paid_key_available':(runtime/'zai-key').exists(),'paid_enabled_by_default':False,'paid_budget_usd':10,'budget':json.loads((runtime/'paid-budget.json').read_text()) if (runtime/'paid-budget.json').exists() else {'reserved_microusd':0,'calls':0}},indent=2)); return
    router=Router(runtime,args.paid)
    if args.probe:
        router.paid=False
        try:
            result=router.complete({'messages':[{'role':'user','content':'Reply exactly READY.'}],'max_tokens':32})
            print(result['choices'][0]['message'].get('content',''))
        except Exception as e:
            sys.exit(str(e) if isinstance(e,RouteError) else 'Readiness probe failed.')
        return
    server=HTTPServer(('127.0.0.1',0),Handler)
    server.router=router; server.secret=secrets.token_urlsafe(32)
    profile=runtime/'profile'; profile.mkdir(exist_ok=True,mode=0o700)
    save(profile/'models.json',{'providers':{'nvidia-first':{'baseUrl':f'http://127.0.0.1:{server.server_port}/v1','api':'openai-completions','apiKey':server.secret,'authHeader':True,'headers':{'Authorization':'Bearer '+server.secret},'compat':{'supportsDeveloperRole':False,'supportsReasoningEffort':False,'supportsStore':False,'maxTokensField':'max_tokens'},'models':[{'id':MODEL,'name':'NVIDIA first; optional capped Z.ai fallback','input':['text'],'contextWindow':16384,'maxTokens':8192,'reasoning':False}]}}})
    save(profile/'settings.json',{'defaultProvider':'nvidia-first','defaultModel':MODEL,'defaultThinkingLevel':'off','retry':{'enabled':False}})
    env=dict(os.environ,PI_CODING_AGENT_DIR=str(profile),PI_OFFLINE='1',NVIDIA_ROUTER_TOKEN=server.secret)
    # Keep vendor keys out of worker subprocesses; only the router needs them.
    for key in ('NVIDIA_API_KEY','ZAI_API_KEY','TOGETHER_API_KEY','OPENROUTER_API_KEY'): env.pop(key,None)
    thread=threading.Thread(target=server.serve_forever,daemon=True); thread.start()
    print('NVIDIA-first ready. Paid fallback '+('ENABLED, $10 conservative lifetime budget.' if args.paid else 'OFF.')+' Sessions saved under .git/pi-nvidia-first/sessions.',file=sys.stderr)
    try:
        code=subprocess.call(['pi','--offline','--provider','nvidia-first','--model',MODEL,'--session-dir',str(runtime/'sessions'),'--no-extensions',*pi_args],cwd=root,env=env)
    finally: server.shutdown(); server.server_close()
    sys.exit(code)

if __name__=='__main__': main()
