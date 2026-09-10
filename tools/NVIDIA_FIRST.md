# NVIDIA-first coding launcher

Run from anywhere:

```sh
python3 /Users/dmytro/github/openttd-rust/tools/nvidia_first.py
```

This launches pi in the OpenTTD Rust repository using NVIDIA Nemotron Super.
The upstream response is buffered, so text appears when generation finishes.
A private, loopback-only gateway preserves the current tool conversation during
inference fallback. It does not restart whole tasks or re-execute previous tools.

## Free mode (default)

- Existing `NVIDIA_API_KEY` environment variable is used. If your terminal does not
  have it, enter the key privately with `--configure-nvidia`.
- One launcher/worker at a time. Three NVIDIA attempts, spaced at least 3 seconds
  apart, with backoff for overload/rate-limit errors. A longer Retry-After pauses
  NVIDIA access; the launcher never repeatedly hammers an overloaded endpoint.
- No paid provider calls by default, even if a paid key exists.
- Authentication and malformed-request failures stop instead of switching providers.
- Sessions and credentials are kept under `.git/pi-nvidia-first/`, outside tracked
  source. Credentials are mode 0600. The router passes only a temporary local token
  to pi; it does not place vendor keys in command arguments or pi config.
- Regular pi settings are untouched. This isolated profile disables automatic
  extension discovery, so existing pi extensions/MCP integrations are not inherited.
- Context is deliberately bounded to 16K advertised tokens / 120KB request bytes;
  output capped at 8192 tokens. Use small tasks and compact when needed. This is not
  a whole-repository ingestion setup. Agent graph access needs a separately loaded
  extension or existing graph CLI; this launcher does not install one.

```sh
python3 tools/nvidia_first.py --status
python3 tools/nvidia_first.py --probe
python3 tools/nvidia_first.py --configure-nvidia
python3 tools/nvidia_first.py --continue
```

Use normal pi flags after the launcher options, for example `--tools read,grep,find,ls`.
Instruct the worker to use `python3 tools/ctx.py` (`task`, `check`, `test`, `slice`)
and consult `memory/` shards to stay within the 16K token boundary.
In a task prompt, explicitly require bounded changes, regression tests, and the
locked workspace checks described by AGENTS.md. No repair job starts automatically.

## Prepared paid fallback — OFF until explicitly enabled

Provider: Z.ai **pay-as-you-go**, `glm-5.3-flash`, standard endpoint.
Do not use the Z.ai Coding Plan endpoint or assume subscription credits cover this.
First create/fund a provider account with no more than $10 prepaid and no auto top-up.
Then enter its key locally:

```sh
python3 tools/nvidia_first.py --configure-paid
python3 tools/nvidia_first.py --paid
```

Published rates checked 2026-09-10: $0.15/M input, $0.50/M output.
https://docs.z.ai/guides/overview/pricing

The launcher applies a conservative **$10 lifetime reservation budget**, not an
account-wide billing limit. It reserves $0.16 durably BEFORE each paid attempt,
including failed/ambiguous attempts, and never refunds reservations. This allows
at most 62 paid requests ($9.92 reserved). Reservation assumes current rates, a
text-only bounded request and output <=8192 tokens. Actual cost should be much
lower; this deliberate over-reservation avoids relying on missing usage reports.
Provider pricing changes or charges made outside this launcher are not covered.
The provider's own prepaid balance is the final financial guard. The budget never
resets on restart or at month boundaries. Do not delete its ledger to restart it.
There is no automatic account creation, credit purchase or payment setup.

Paid model responses are identified on stderr; pi's selected model remains the
router alias, so its built-in model cost display is not authoritative. Use
`--status` for reservations and the provider dashboard for actual billed usage.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/test_nvidia_first.py
```

Rollback: stop the launcher and run ordinary `pi`. Remove the two Python scripts,
this document and `.git/pi-nvidia-first` only if you also want to delete saved
sessions, keys and the budget record.
