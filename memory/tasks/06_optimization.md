# Task 6: Measure and Optimize (`memory/tasks/06_optimization.md`)

## Status: COMPLETED

## Problem Description
- Key allocations and lookups occur repeatedly inside `tick()`.
- Tile memory footprint needs accurate measurement (current claims of 16/20 bytes need validation).
- Benchmarking harness is needed to measure tick duration across scale.

## Objective & Scope
1. **Benchmark Suite**: Create release benchmarks covering fleet sizes (10, 100, 1,000 vehicles) and map sizes (64x64, 256x256).
2. **Eliminate Tick Allocations**: Cache iteration buffers; replace per-tick heap allocations with reusable structures.
3. **Memory Layout Validation**: Measure exact `size_of::<Tile>()` and map density.
4. **Deterministic Invariant**: Profile and optimize without changing simulation trace output.

## Target Files
- [`crates/transport-sim/benches/`](crates/transport-sim/benches/)
- [`crates/transport-world/src/map.rs`](crates/transport-world/src/map.rs)

## Acceptance Criteria & Tests
- Benchmarks establish baseline latency and memory usage.
- Zero per-tick heap allocations in steady-state vehicle transport loop.
