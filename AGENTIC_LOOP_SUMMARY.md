# Coding-agent handoff and simulation status

Baseline reviewed 2026-09-10. This file describes implementation status, not a working transport simulation.

## Verified baseline

- Six Rust crates compile with `cargo check --workspace --locked`.
- `cargo test --workspace --locked` passes two tests, both map tests.
- Existing warnings: unused imports/variables, workspace resolver default and unsupported `workspace.dev-dependencies` manifest key.
- `.github/workflows/rust.yml` adds these checks for pushes and pull requests to main. It has been authored locally; no hosted CI run is claimed.
- Lean contains proof placeholders. C++ parity and the reference fixture runner have not been verified.

## Simulation status

`Simulator::tick()` is a simulation loop, not a coding-agent supervisor. It drains queued commands (currently discarding their results), mutates order indexes without executing orders, calls cargo/person placeholders and increments the tick counter. Only CreateCompany is supported. Vehicles do not move. Station lookup returns None. See the repair plan for reproduced defects and acceptance gates.

## External worker orchestration

The coordinator runs pi workers sequentially with bounded file ownership, saved sessions and result logs. The configured Ollama endpoint failed to connect; NVIDIA-backed pi completed readiness-01. Superset was not logged in. Machine-local task state is in `.git/pi-orchestration/state.json`; this is not a portable service or unattended scheduler.

## Next tasks

Follow [the repair plan](specs/003-correctness-and-performance-plan.md): command outcomes, correct orders/determinism, state validation, recoverable snapshots, working transport slice, then measured optimization. Each repair needs regression tests and coordinator review before dependent work starts. Preserve the user's existing uncommitted and untracked work.

## Latest supervised run

Readiness-01 completed and was reviewed. Commands-01 is blocked after repeated NVIDIA service overloads across Nemotron Super and Ultra. The incomplete draft and sessions are archived under `.git/pi-orchestration/commands/`; the exact pre-worker command source was restored using its recorded hash. No repair is claimed. No worker remains running. Retry commands-01 with pi and NVIDIA Nemotron when the endpoint is available; dependent tasks remain pending.
