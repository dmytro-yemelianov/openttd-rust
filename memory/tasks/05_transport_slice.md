# Task 5: One Working Transport Slice (`memory/tasks/05_transport_slice.md`)

## Status: COMPLETED

## Problem Description
- Vehicles do not currently move along paths.
- Station lookup returns `None` (`station_at` is an empty stub).
- Dock construction, ship purchasing, schedules, arrival, and cargo transfer are unimplemented.

## Objective & Scope
1. **Dock & Station Indexing**: Maintain an authoritative `tile -> StationID` lookup on build/remove/restore.
2. **Ship Slice**: Implement dock placement, ship purchase, simple schedule assignment, and discrete movement.
3. **Cargo Transfer**: Capacity-limited loading at origin, transport, and unloading at destination with payment.
4. **Conservation Invariant**: Cargo is never created from thin air or destroyed on vehicle movement.

## Target Files
- [`crates/transport-world/src/map.rs`](crates/transport-world/src/map.rs)
- [`crates/transport-sim/src/lib.rs`](crates/transport-sim/src/lib.rs)
- [`crates/transport-types/src/enum_.rs`](crates/transport-types/src/enum_.rs)

## Acceptance Criteria & Tests
- End-to-end integration test: Ship moves between dock A and dock B, loads cargo at A, delivers at B, and company receives calculated revenue.
- Test cargo conservation across loading, transport, and unloading.
