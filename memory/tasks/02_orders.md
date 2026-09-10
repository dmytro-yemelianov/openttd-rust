# Task 2: Orders and Deterministic Ticks (`memory/tasks/02_orders.md`)

## Status: PENDING (Depends on Task 1)

## Problem Description
- `Simulator::tick()` mutates order indexes blindly without executing orders or checking valid bounds.
- Upstream order progression had an arbitrary ID threshold (e.g. 999 limit).
- Empty order lists cause undefined index behavior; vehicle processing order depends on `HashMap` iteration.

## Objective & Scope
1. **Station Destinations**: Add explicit station/depot destinations to orders.
2. **Explicit Execution States**: Waiting, travel, transfer, and completion states.
3. **Remove Arbitrary Thresholds**: Support valid IDs of any representable index.
4. **List Invariants**: Empty lists must remain idle; advance order index only upon verified completion.
5. **Deterministic Processing Order**: Vehicles must be processed in sorted `VehicleID` order, never raw hash order.

## Target Files
- [`crates/transport-sim/src/lib.rs`](crates/transport-sim/src/lib.rs)
- [`crates/transport-types/src/id.rs`](crates/transport-types/src/id.rs)
- [`crates/transport-types/src/enum_.rs`](crates/transport-types/src/enum_.rs)

## Acceptance Criteria & Tests
- Regression test: Vehicle with empty order list remains idle without panicking or mutating index.
- Regression test: Vehicle order transitions only when arrival/loading conditions complete.
- Regression test: Stable execution order across identical seeds and map states.
