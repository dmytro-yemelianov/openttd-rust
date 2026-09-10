#!/usr/bin/env python3
"""Token-efficient context and verification helper for openttd-rust.

Designed to maximize signal and minimize token consumption for AI agents.
Zero external dependencies (pure Python 3 standard library).
"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

REPO_ROOT = Path(__file__).resolve().parent.parent
MEMORY_DIR = REPO_ROOT / "memory"
STATE_FILE = MEMORY_DIR / "state.json"
TASKS_DIR = MEMORY_DIR / "tasks"


def run_cmd(cmd, cwd=REPO_ROOT, check=False):
    """Run shell command and return (returncode, stdout, stderr)."""
    res = subprocess.run(
        cmd,
        cwd=str(cwd),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        shell=isinstance(cmd, str),
    )
    if check and res.returncode != 0:
        print(f"Command failed with code {res.returncode}: {cmd}", file=sys.stderr)
        if res.stderr:
            print(res.stderr, file=sys.stderr)
        sys.exit(res.returncode)
    return res.returncode, res.stdout, res.stderr


def load_state():
    """Load project memory state.json."""
    if not STATE_FILE.exists():
        return {}
    try:
        return json.loads(STATE_FILE.read_text())
    except Exception as e:
        print(f"Warning: Failed to parse {STATE_FILE}: {e}", file=sys.stderr)
        return {}


def save_state(state):
    """Save project memory state.json."""
    STATE_FILE.write_text(json.dumps(state, indent=2) + "\n")


def cmd_status(args):
    """Print high-signal, compact project status (~10 lines)."""
    state = load_state()
    active_task = state.get("active_task", "unknown")
    task_info = state.get("tasks", {}).get(active_task, {})
    
    # Git info
    code, stdout, _ = run_cmd(["git", "rev-parse", "--short", "HEAD"])
    commit = stdout.strip() if code == 0 else "unknown"
    code, stdout, _ = run_cmd(["git", "branch", "--show-current"])
    branch = stdout.strip() if code == 0 else "unknown"
    code, stdout, _ = run_cmd(["git", "status", "--porcelain"])
    dirty_count = len(stdout.strip().splitlines()) if stdout.strip() else 0

    print("=== openttd-rust Status ===")
    print(f"Git: {branch} @ {commit} ({dirty_count} uncommitted file changes)")
    print(f"Active Task: {active_task} -> {task_info.get('name', 'N/A')}")
    print(f"Task Status: {task_info.get('status', 'unknown')}")
    if task_info.get("blocked_reason"):
        print(f"Blocker Note: {task_info.get('blocked_reason')}")
    print(f"Target Crates: {', '.join(task_info.get('target_crates', []))}")
    print("Project Memory: memory/README.md, memory/state.json, memory/architecture.md")
    print("Graph Memory: .codebase-memory/graph.db.zst (indexed)")
    print("Tip: Run 'python3 tools/ctx.py task' to view active task requirements.")


def resolve_task_file(task_arg, state):
    """Resolve a task identifier (e.g. 1, 01, commands, task_01_commands) to a markdown file."""
    if not task_arg:
        active = state.get("active_task")
        if active and active in state.get("tasks", {}):
            card = state["tasks"][active].get("card")
            if card:
                return REPO_ROOT / card
        task_arg = "01"

    # Try exact match in tasks dict
    tasks = state.get("tasks", {})
    if task_arg in tasks:
        card = tasks[task_arg].get("card")
        if card:
            return REPO_ROOT / card

    # Search in tasks dir
    task_files = sorted(TASKS_DIR.glob("*.md"))
    arg_lower = task_arg.lower()
    for tf in task_files:
        stem = tf.stem.lower()
        if arg_lower in stem or stem.startswith(arg_lower.zfill(2)):
            return tf

    return None


def cmd_task(args):
    """Print bounded task requirements card (< 50 lines)."""
    state = load_state()
    task_file = resolve_task_file(args.task_id, state)
    if not task_file or not task_file.exists():
        print(f"Task '{args.task_id}' not found. Available tasks:", file=sys.stderr)
        for tf in sorted(TASKS_DIR.glob("*.md")):
            print(f"  - {tf.stem}", file=sys.stderr)
        sys.exit(1)

    print(task_file.read_text().strip())


def cmd_check(args):
    """Run cargo check with warning suppression (~95% token savings)."""
    cmd = ["cargo", "check", "--locked"]
    if args.crate:
        cmd.extend(["-p", args.crate])
    else:
        cmd.append("--workspace")

    code, stdout, stderr = run_cmd(cmd)

    if args.all_warnings or code != 0:
        # If compilation failed, extract only errors and offending snippets
        lines = (stderr or "").splitlines()
        errors = []
        in_error = False
        warning_count = 0
        error_count = 0

        for line in lines:
            if line.startswith("error:") or line.startswith("error["):
                in_error = True
                error_count += 1
                errors.append(line)
            elif line.startswith("warning:"):
                in_error = False
                warning_count += 1
                if args.all_warnings:
                    errors.append(line)
            elif in_error:
                # Keep error message context (code snippet and pointer)
                if line.startswith(" -->") or line.strip().startswith("|") or line.strip().startswith("="):
                    errors.append(line)
                elif line.strip() == "":
                    in_error = False

        if error_count > 0:
            print(f"✗ cargo check failed with {error_count} error(s):")
            print("\n".join(errors))
            sys.exit(code)
        elif args.all_warnings:
            print(stderr)
            sys.exit(code)

    if code == 0:
        # Count suppressed warnings for confirmation
        warnings = len([l for l in (stderr or "").splitlines() if l.startswith("warning:")])
        print(f"✓ cargo check passed (0 errors, {warnings} baseline warnings suppressed)")
    else:
        print(stderr)
        sys.exit(code)


def cmd_test(args):
    """Run cargo test with compact failure-focused output (~90% token savings)."""
    cmd = ["cargo", "test", "--locked"]
    if args.crate:
        cmd.extend(["-p", args.crate])
    else:
        cmd.append("--workspace")
    if args.test:
        cmd.extend(["--", args.test])

    code, stdout, stderr = run_cmd(cmd)

    if args.verbose:
        print(stdout)
        if stderr:
            print(stderr, file=sys.stderr)
        sys.exit(code)

    # Parse test outcomes
    all_output = stdout + "\n" + (stderr or "")
    passed_matches = re.findall(r"test result: ok\. (\d+) passed; 0 failed", all_output)
    total_passed = sum(int(m) for m in passed_matches)
    
    # Check for failures
    failure_matches = re.findall(r"test (\S+) \.\.\. FAILED", all_output)
    
    if code == 0 and not failure_matches:
        print(f"✓ cargo test passed ({total_passed} tests passed, 0 failures)")
    else:
        print(f"✗ cargo test failed ({len(failure_matches)} failed):")
        # Extract failures section
        in_failures = False
        fail_lines = []
        for line in all_output.splitlines():
            if line.startswith("failures:"):
                in_failures = True
            if in_failures:
                fail_lines.append(line)
                if line.startswith("test result: FAILED"):
                    break
        if fail_lines:
            print("\n".join(fail_lines))
        else:
            print("\n".join(all_output.splitlines()[-30:]))
        sys.exit(code or 1)


def cmd_slice(args):
    """Extract a small bounded chunk of code by symbol name or line range."""
    path = REPO_ROOT / args.file
    if not path.exists():
        print(f"File not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    lines = path.read_text(errors="replace").splitlines()
    target = args.target.strip()

    # Case 1: Line range e.g. "20-50" or "42"
    range_match = re.match(r"^(\d+)(?:-(\d+))?$", target)
    if range_match:
        start = max(1, int(range_match.group(1)))
        end = int(range_match.group(2)) if range_match.group(2) else start + args.max_lines - 1
        end = min(len(lines), end)
        print(f"=== {args.file} (lines {start}-{end} of {len(lines)}) ===")
        for i in range(start, end + 1):
            print(f"{i:4d}: {lines[i - 1]}")
        return

    # Case 2: Symbol search (e.g. "fn process_command", "struct Map", "Command")
    pattern = re.compile(r"\b" + re.escape(target) + r"\b")
    match_idx = None
    for idx, line in enumerate(lines):
        if pattern.search(line):
            match_idx = idx
            break

    if match_idx is None:
        # Fallback substring match
        for idx, line in enumerate(lines):
            if target in line:
                match_idx = idx
                break

    if match_idx is None:
        print(f"Symbol/pattern '{target}' not found in {args.file}", file=sys.stderr)
        sys.exit(1)

    # Start a few lines before to catch docstrings / attributes
    start_idx = match_idx
    while start_idx > 0 and (lines[start_idx - 1].strip().startswith("///") or lines[start_idx - 1].strip().startswith("#[")):
        start_idx -= 1

    end_idx = min(len(lines), start_idx + args.max_lines)

    # Try to balance braces if searching for a block
    brace_count = 0
    found_brace = False
    for i in range(start_idx, len(lines)):
        line = lines[i]
        brace_count += line.count("{") - line.count("}")
        if "{" in line:
            found_brace = True
        if found_brace and brace_count <= 0:
            end_idx = min(len(lines), i + 1)
            break
        if i - start_idx >= args.max_lines:
            break

    print(f"=== {args.file} (symbol: '{target}', lines {start_idx + 1}-{end_idx}) ===")
    for i in range(start_idx, end_idx):
        print(f"{i + 1:4d}: {lines[i]}")


def cmd_symbols(args):
    """List struct, enum, trait, and fn declarations with line numbers in a file or directory."""
    path = REPO_ROOT / args.path
    if path.is_file():
        files = [path]
    elif path.is_dir():
        files = sorted(path.rglob("*.rs"))
    else:
        print(f"Path not found: {args.path}", file=sys.stderr)
        sys.exit(1)

    sym_pattern = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?(fn|struct|enum|trait|type|impl)\s+([A-Za-z0-9_]+)")
    for f in files:
        rel = f.relative_to(REPO_ROOT)
        lines = f.read_text(errors="replace").splitlines()
        found = []
        for idx, line in enumerate(lines):
            m = sym_pattern.match(line)
            if m:
                kind, name = m.group(1), m.group(2)
                found.append(f"{idx + 1:4d}: {kind:6s} {name}")
        if found:
            print(f"--- {rel} ---")
            for item in found:
                print(item)


def cmd_update_task(args):
    """Update task status in memory/state.json."""
    state = load_state()
    tasks = state.setdefault("tasks", {})
    task_id = args.task_id
    if task_id not in tasks:
        # Try matching by prefix/number
        matches = [k for k in tasks if task_id in k]
        if len(matches) == 1:
            task_id = matches[0]
        else:
            print(f"Unknown task '{task_id}'. Valid tasks: {list(tasks.keys())}", file=sys.stderr)
            sys.exit(1)

    tasks[task_id]["status"] = args.status
    if args.blocker is not None:
        tasks[task_id]["blocked_reason"] = args.blocker if args.blocker != "" else None
    if args.set_active:
        state["active_task"] = task_id

    save_state(state)
    print(f"✓ Updated task '{task_id}' -> status: {args.status}")
    if args.set_active:
        print(f"✓ Set active task -> {task_id}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    # status
    p_status = subparsers.add_parser("status", help="Show compact project & task status")
    p_status.set_defaults(func=cmd_status)

    # task
    p_task = subparsers.add_parser("task", help="Display bounded task card")
    p_task.add_argument("task_id", nargs="?", default=None, help="Task ID or number (default: active task)")
    p_task.set_defaults(func=cmd_task)

    # check
    p_check = subparsers.add_parser("check", help="Run cargo check with warning suppression")
    p_check.add_argument("-p", "--crate", help="Specific crate to check")
    p_check.add_argument("--all-warnings", action="store_true", help="Do not suppress baseline warnings")
    p_check.set_defaults(func=cmd_check)

    # test
    p_test = subparsers.add_parser("test", help="Run cargo test with compact output")
    p_test.add_argument("-p", "--crate", help="Specific crate to test")
    p_test.add_argument("-t", "--test", help="Specific test pattern to run")
    p_test.add_argument("-v", "--verbose", action="store_true", help="Show full cargo test output")
    p_test.set_defaults(func=cmd_test)

    # slice
    p_slice = subparsers.add_parser("slice", help="View targeted code chunk without full-file dump")
    p_slice.add_argument("file", help="Relative file path")
    p_slice.add_argument("target", help="Symbol name (e.g. 'fn tick') or line range ('20-45')")
    p_slice.add_argument("-n", "--max-lines", type=int, default=50, help="Maximum lines to print (default 50)")
    p_slice.set_defaults(func=cmd_slice)

    # symbols
    p_sym = subparsers.add_parser("symbols", help="Outline symbols in a file or directory")
    p_sym.add_argument("path", help="Relative path to file or directory")
    p_sym.set_defaults(func=cmd_symbols)

    # update-task
    p_up = subparsers.add_parser("update-task", help="Update task state in memory/state.json")
    p_up.add_argument("task_id", help="Task ID or number")
    p_up.add_argument("--status", required=True, choices=["pending", "ready_for_retry", "in_progress", "completed", "blocked"])
    p_up.add_argument("--blocker", default=None, help="Blocker explanation (empty string to clear)")
    p_up.add_argument("--set-active", action="store_true", help="Set this task as active")
    p_up.set_defaults(func=cmd_update_task)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
