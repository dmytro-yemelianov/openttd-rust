# Upstream Safeguards Checklist (`memory/safeguards.md`)

When porting or repairing subsystems from upstream OpenTTD, NEVER copy upstream limitations mechanically. Verify these safeguards for every pull request:

## 1. Determinism (Critical)
- **No HashMap Iteration Order**: Never iterate over raw `HashMap` keys or values to drive simulation updates or ties. Always sort by ID (e.g. `VehicleID`, `CompanyID`) or maintain stable ordered collections (`BTreeMap` / indexed vectors).
- **No Wall-Clock Reliance**: Never allow system time, background thread timings, or worker latency to dictate simulation outcomes.
- **Trace Equivalence**: Repeated runs from identical inputs must produce bitwise/logical identical states. Save/reload cycles must match continuous execution.

## 2. Arithmetic & Resource Bounds
- **Checked Arithmetic**: Use checked or saturating math for `Money`, `CargoAmount`, and `Ticks`. Never panic or silently wrap on overflow/underflow.
- **ID Safety**: Allocation must never return `INVALID` or silently overwrite existing entities on exhaustion. Handle exhausted ID spaces with explicit errors.
- **Memory & Work Bounds**: All routing, queueing, and search algorithms must have explicit hard bounds on iteration count and memory footprint.

## 3. Caches as Derived State
- Caches are always subordinate to authoritative state.
- Include topology revisions and ruleset versions in cache validity keys.
- Must verify cached vs uncached equivalence before enabling any pathfinding cache.
- Rebuild derived indexes from scratch upon snapshot restoration.

## 4. Observable Failures
- Never swallow errors into no-op `Success`.
- Return typed, descriptive failure enums (`CommandResult::Failure(...)`).
- Failed commands must mutate zero state.
