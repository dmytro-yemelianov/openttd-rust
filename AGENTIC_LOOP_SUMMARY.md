# Coding-Agent Handoff and Simulation Status

Updated 2026-09-11. This file documents the production status of the `openttd-rust` simulation ecosystem.

## Verified Production Baseline

- **12 Workspace Crates**:
  - `transport-types`: Fixed-width IDs, arithmetic types, unit primitives.
  - `transport-world`: Spatial map storage, entity pools, towns, buildings, stations.
  - `transport-sim`: Deterministic Capability Microkernel (DCM), 5-phase deterministic tick cycle, ship transport slice, A* water pathfinding.
  - `transport-people`: Tiered population, commute journeys, turnstiles, anti-laundering trip economics.
  - `transport-api`: Wait-free ring buffer pub/sub, total-order command ingress bus.
  - `transport-orm`: WAL crash recovery, non-blocking asynchronous snapshotting, generational save/restore.
  - `transport-render`: Agnostic presentation layer, 2:1 dimetric isometric/orthographic camera, Painter's depth sorting, sub-tick interpolation, software framebuffer (BMP/PPM export).
  - `transport-tools`: Interaction controller, capability-aware dry-run validation, catchment overlays, ghost previews.
  - `transport-scenario`: Procedural integer map generator, waterway reachability flood-fill, milestone objectives.
  - `transport-dashboard`: Hierarchical RRD ring buffer, multi-scale financial ledger, ASCII sparklines, Prometheus/JSON exporters.
  - `transport-cli`: Runnable CLI binary with live ANSI viewport, telemetry dashboard, interactive/demo modes.
  - `transport-wasm`: WebAssembly bridge and HTML5 Canvas host.

- **78/78 Passing Tests**: `cargo test --workspace --locked` passes 100% across all crates.
- **Zero Compiler & Clippy Warnings**: Clean build with `cargo clippy --workspace --locked --all-targets -- -D warnings`.
- **Formal Verification in Lean 4**: 14/14 proof modules compile cleanly with `lake build` (zero errors, zero `sorry`).
- **Automated CI/CD**: `.github/workflows/rust.yml` tests and enforces Rust compilation, Clippy lints, unit/integration tests, and Lean 4 formal proofs on all pushes and PRs.

## Simulation Status

The simulation operates as a deterministic discrete-tick microkernel:
- Strictly decoupled: The simulation core has **zero** graphics or GUI dependencies.
- Movement, logistics, and passenger commutes execute deterministically across ticks.
- Commands are ordered, validated with non-mutating dry-run tokens, and journaled to WAL.
- Presentation clients (`transport-render`, `transport-cli`, `transport-wasm`) consume egress state read-only and emit staged commands via `transport-api`.

## Running the Simulation

```bash
# Run headless simulation for 100 ticks with dashboard output:
cargo run -p transport-cli -- --ticks 100

# Run live terminal demo with animated ANSI viewport:
cargo run -p transport-cli -- --demo

# Export Prometheus metrics:
cargo run -p transport-cli -- --prometheus

# Export JSON telemetry:
cargo run -p transport-cli -- --json

# Run all workspace tests:
cargo test --workspace --locked

# Build Lean 4 formal proofs:
cd lean && lake build
```
