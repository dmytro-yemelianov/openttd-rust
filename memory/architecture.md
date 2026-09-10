# Architecture Map (`memory/architecture.md`)

## Crate Dependency Hierarchy

```
       transport-api  (Facade / Event Dispatch)
         /         \
transport-people    transport-orm  (Snapshots & Persistence)
         \         /
        transport-sim      (Tick Loop & Command Queue)
             |
       transport-world     (Map, Tiles, Entities: Company, Vehicle, Station)
             |
       transport-types     (IDs, Units, Enums, Errors)
```

## Crate Responsibilities & Boundaries

| Crate | Responsibility | Key Symbols / Entry Points | Depends On |
|---|---|---|---|
| [`transport-types`](crates/transport-types/) | Fundamental scalar types, newtypes, units, error enum, IDs | `CompanyID`, `VehicleID`, `TileIndex`, `Money`, `Ticks`, `Error` | None |
| [`transport-world`](crates/transport-world/) | Spatial grid, tile storage, entities, definition tables | `Map`, `Tile`, `Company`, `Vehicle`, `Station`, `Engine`, `Cargo` | `transport-types` |
| [`transport-sim`](crates/transport-sim/) | Deterministic tick progression, command queues, order loop | `Simulator`, `World`, `Command`, `CommandResult`, `tick()` | `types`, `world` |
| [`transport-orm`](crates/transport-orm/) | Durable snapshot serialization and state envelopes | `save_snapshot()`, `load_snapshot()`, `Error` | `types`, `world`, `sim` |
| [`transport-people`](crates/transport-people/) | Persistent travelers, journeys, boarding contracts | `PeopleSimulator`, `Person`, `Journey`, `Assignment` | `types`, `world`, `sim` |
| [`transport-api`](crates/transport-api/) | Public facade, external event publication | `Event` | All crates |

## Upstream Oracle Boundary

- Upstream OpenTTD is located at `../OpenTTD` (referenced in [`upstream.json`](upstream.json)).
- Queries to upstream C++ logic should be routed via `codebase-memory-mcp` using `project_name="OpenTTD"`.
- Do not import C++ files directly into this workspace.

## Deterministic Capability Microkernel (DCM) Implementation

- **Kernel Coordinator ([crates/transport-sim/src/kernel/mod.rs](crates/transport-sim/src/kernel/mod.rs))**:
  - Authoritative coordinator managing the single-source-of-truth `World`, deterministic 5-phase dispatch (`Ingress`, `Solvers`, `Drivers`, `Commit`, `Egress`), and transaction commit.
  - Formally modeled and proven monotonic & deterministic in [lean/src/Transport.lean](lean/src/Transport.lean).
- **Capability Tokens ([crates/transport-types/src/capability.rs](crates/transport-types/src/capability.rs))**:
  - `CapabilityToken::System`, `CapabilityToken::Company(CompanyID)`, `CapabilityToken::Observer`.
  - Staged intents (`KernelIntent`) must present valid capabilities before mutating company funds, vehicle coordinates, or station goods during `Phase::Commit`.
- **Subsystem Drivers ([crates/transport-sim/src/drivers/](crates/transport-sim/src/drivers/))**:
  - `MovementDriver`: Vehicle schedule traversal and single-tile coordinate stepping.
  - `LogisticsDriver`: Station docking, cargo loading, unloading, and capacity limits.
  - `EconomyDriver`: Vehicle running costs and periodic financial balancing.
- **Fault Isolation & Rollback**:
  - Subsystem driver errors abort intent staging without polluting or corrupting authoritative `World` state.
- **High Throughput**:
  - Validated up to **1.65M ticks/s** (small fleets) and **17,200+ ticks/s** (1,000-vehicle fleets) with zero per-tick heap allocations.
