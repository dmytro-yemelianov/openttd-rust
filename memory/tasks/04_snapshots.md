# Task 4: Recoverable Snapshots (`memory/tasks/04_snapshots.md`)

## Status: COMPLETED

## Problem Description
- `transport-orm` currently has stubbed/untested save and load routines.
- No versioned envelope, atomic file replacement, or protection against interrupted writes.
- Derived indexes are not properly rebuilt on state restoration.

## Objective & Scope
1. **Versioned Envelope**: Encase world state, pending commands, counters, and rulesets in a versioned envelope.
2. **Atomic Generational Persistence**: Write new generations to temporary files, flush to disk, then atomically rename/publish. Retain previous valid generation.
3. **Rebuild Derived Indexes**: Recompute tile lookups and spatial indexes upon load.
4. **Validation on Restore**: Validate all invariants before swapping live simulator state with loaded state.

## Target Files
- [`crates/transport-orm/src/lib.rs`](crates/transport-orm/src/lib.rs)
- [`crates/transport-sim/src/lib.rs`](crates/transport-sim/src/lib.rs)

## Acceptance Criteria & Tests
- Regression test: Continuous 100-tick run produces state identical to a run saved at tick 50 and restored.
- Regression test: Interrupted/corrupt save file does not destroy previous valid snapshot.
