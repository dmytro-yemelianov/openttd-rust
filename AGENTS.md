# Agent Repair Guidelines for openttd-rust

## Graph Discovery Rule
- Prefer `codebase-memory-mcp` tools: `search_graph`, `trace_path`, `get_code_snippet`, `query_graph`, `get_architecture`
- Fallback to `rg` (ripgrep) when graph unavailable/insufficient or searching literals/configs

## Repair Requirements
- Read the repair plan (specs/003-correctness-and-performance-plan.md) before starting
- Work on bounded tasks as defined in the plan
- Preserve unrelated changes (do not modify files outside the task scope)
- Include regression tests for behavior repairs
- Ensure deterministic state transitions
- Apply upstream safeguards (do not carry identified limitations into Rust by mechanical translation)

## Project Memory and Token Efficiency
- Use sharded project memory under [memory/](memory/) instead of reading large specs into context:
  - Task cards: [memory/tasks/](memory/tasks/) (~30 lines each)
  - Data model: [memory/data_model.md](memory/data_model.md)
  - Architecture: [memory/architecture.md](memory/architecture.md)
  - State of tasks: [memory/state.json](memory/state.json)
- Use [tools/ctx.py](tools/ctx.py) to minimize token consumption:
  - `python3 tools/ctx.py status` -> ~10-line high-signal project snapshot
  - `python3 tools/ctx.py task` -> displays active bounded task requirements
  - `python3 tools/ctx.py check` -> suppresses 20+ baseline compiler warnings (~95% token savings)
  - `python3 tools/ctx.py test` -> suppresses noisy test banners (~90% token savings)
  - `python3 tools/ctx.py slice <file> <symbol_or_lines>` -> view only relevant symbol/lines without reading whole files
  - `python3 tools/ctx.py symbols <file>` -> 1-line per symbol outline

## Workflow
1. Understand the defect using graph discovery or `tools/ctx.py slice`
2. Implement fix within bounded scope defined in [memory/tasks/](memory/tasks/)
3. Add regression test verifying the fix
4. Run `python3 tools/ctx.py check` and `python3 tools/ctx.py test` (or cargo check/test directly)
5. Ensure no new warnings are introduced (existing warnings documented below)

## Handoff and supervision
- Use pi workers backed by NVIDIA Nemotron, as requested by the user. Preserve this model preference for follow-up tasks.
- Keep implementation tasks sequential while they share core types. Report changed files, checks and remaining blockers at completion.
- Read AGENTIC_LOOP_SUMMARY.md for current status; compiling placeholders are not completed features.
- The simulation tick loop is separate from external coding-agent orchestration. Current orchestration uses pi directly; Superset was unavailable because it was not logged in.

