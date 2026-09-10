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

## Microkernel / RTOS Architecture Direction

- **Kernel Core (`transport-sim`, `transport-types`)**: Minimal deterministic coordinator managing the single-source-of-truth `World`, monotonic scheduling, and transaction execution. Formally proven with Lean 4.
- **Isolated Services**: Heavy subsystems (asynchronous pathfinding, CargoDist MCF solver, economy cycles, people/agents, rendering/GUI) communicate via typed message queues / capabilities rather than shared mutable pointers.
- **Fault Isolation**: Timeouts and solver budgets in worker services never corrupt the kernel's world state or stall the tick loop.
