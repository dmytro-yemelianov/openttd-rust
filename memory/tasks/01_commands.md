# Task 1: Commands and Observable Failures (`memory/tasks/01_commands.md`)

## Status: READY_FOR_RETRY (Step 1 in Repair Sequence)

## Problem Description
- `Simulator::tick()` currently drains queued commands and discards their results.
- Immediate execution (`process_command`) and queued execution (`queue_command`) follow divergent paths.
- Only `CreateCompany` is partially implemented; other commands are no-ops or panics.
- Unsupported commands return success or silently drop without mutation or error reporting.

## Objective & Scope
1. **Unify Command Handling**: Unify immediate and queued command dispatch into one authoritative handler.
2. **Stable Sequence IDs**: Give every command submission a deterministic sequence ID.
3. **Typed Outcomes**: Replace `CommandResult::Failure(String)` with a strongly-typed failure enum, and return created entity IDs on success.
4. **Ordering**: Preserve strict FIFO for equal priorities; record effective execution order.
5. **No Mutation on Failure**: If a command fails or is unsupported, state must remain completely unmutated.

## Target Files
- [`crates/transport-sim/src/lib.rs`](crates/transport-sim/src/lib.rs)
- [`crates/transport-api/src/lib.rs`](crates/transport-api/src/lib.rs)

## Acceptance Criteria & Tests
- Regression test: Submit an unsupported command -> verify typed failure is returned and `World` state is unchanged.
- Regression test: Queue multiple commands with varying priorities -> verify exact deterministic execution sequence.
- Run: `python3 tools/ctx.py test --crate transport-sim`
