# OpenTTD-Inspired Engine Implementation Plan

Based on the specifications in `specs/001-port-scope-and-estimate.md` and `specs/002-ontology-and-data-model.md`.

## Phase 0: Preparation and Behavioral Reference (A0 - Ontology, compatibility contract and C++ oracle)

### Goals:
- Establish reproducible C++ behavioral reference
- Freeze fixtures and compatibility scope
- Create ontology for the ship-and-crew subset
- Build upstream runner and normalized state export

### Steps:
1. **Clone and build upstream OpenTTD reference** (if not already built)
   ```bash
   cd /Users/dmytro/github/OpenTTD
   mkdir build && cd build
   cmake ..
   make -j$(sysctl -n hw.ncpu)
   ```
2. **Create a simple scenario** for testing:
   - 256x256 map
   - 2 docks connected by water
   - 1 ship
   - 10 named people
   - Seven-operational-day scenario
3. **Export canonical state traces** from C++ reference:
   - Position, orders, cargo, clocks, PRNG state at regular ticks
   - Focus on ship movement, loading/unloading, person journeys
4. **Define ontology subset** for ship-and-crew:
   - Entities: World, Tile, Dock (as Station), Ship (Vehicle), Cargo, Person, Journey
   - Relationships: Company owns Vehicles, Stations maintain Goods Entities, etc.
5. **Create normalized state export format** (JSON) for comparison

## Phase 1: Foundation (A1 - Foundation)

### Goals:
- Implement typed IDs, arenas, map accessors, clocks, PRNG, command context
- Minimal entity lifecycle
- Primitive parity fixtures

### Steps:
1. **Create Rust workspace** with crates:
   - `transport-types`: IDs, numeric units, primitive enums, errors
   - `transport-world`: Map, entity storage, definitions, references
   - `transport-sim`: Commands, ordered ticks, construction, movement, economy, cargo
   - `transport-people`: Optional persistent person/journey extension
   - `transport-api`: Versioned commands, projections, event transport
2. **Implement core data models** based on SPEC-002:
   - Entity ID patterns (CompanyID, VehicleID, StationID, etc.)
   - Tile storage with packed representation
   - Entity pools using typed arenas
   - Clock and PRNG systems matching upstream behavior
3. **Implement command system**:
   - Command context with actor/context validation
   - Command queue and execution
4. **Create foundation fixtures**:
   - ID generation and validity
   - Tile accessors (encode/decode)
   - PRNG sequence reproducibility
   - Date/time conversions

## Phase 2: Stations, Orders and Cargo (A2 - Stations, orders and cargo)

### Goals:
- Implement docks, shared schedules, cargo ownership/split/merge
- Loading/unloading and capacity
- Conservation/roundtrip fixtures

### Steps:
1. **Implement Station/GoodsEntry model**:
   - StationID, GoodsEntry per cargo type
   - Rating, acceptance, cargo lists
2. **Implement OrderList and Order**:
   - Shared order lists, vehicle references
   - Order types: Go to station, wait for cargo, etc.
3. **Implement CargoPacket model**:
   - Split/merge operations conserving count and payment
   - Source, next-hop, distance/payment data
4. **Implement loading/unloading logic**:
   - Station acceptance ratings
   - Cargo transfer between vehicle and station
5. **Create cargo conservation fixtures**:
   - Split/merge conserve total cargo
   - Round-trip journey with loading/unloading

## Phase 3: Ship Movement Slice (A3 - Ship movement slice)

### Goals:
- Fixed water network, ship movement/pathfinding
- Dock interactions
- C++/Rust trace comparisons within declared scope

### Steps:
1. **Implement map and pathfinding**:
   - Water tile network for ship movement
   - Simple pathfinding (A* or Dijkstra) for ship navigation
2. **Implement Vehicle and Engine models**:
   - Engine ID describing vehicle model
   - Vehicle state: position, velocity, orders, cargo
   - Ship-specific movement (water tiles)
3. **Implement movement tick**:
   - Process vehicle movement each tick
   - Path following and dock arrival detection
4. **Implement dock interaction**:
   - Automatic orders for loading/unloading at docks
   - Capacity constraints
5. **Create movement fixtures**:
   - Ship travels from dock A to dock B
   - Compare tick-by-tick traces with C++ reference
   - Verify arrival times, cargo delivered

## Phase 4: People and Host Integration (A4 - People and host integration)

### Goals:
- Persistent identity, journeys, reservation/manifest policy
- Asynchronous authorization inputs, denial consequences
- Person invariants

### Steps:
1. **Implement Person entity**:
   - Stable external ID, current location, activity
   - Links to assignments/journeys
2. **Implement Location enum**:
   - At site, walking leg, aboard vehicle, outside world
   - Aboard carries vehicle ID and journey-leg ID
3. **Implement Journey**:
   - Person, origin, destination, purpose, ordered legs and state
   - Boarding, traveling, transferring, completing
4. **Implement SeatReservation and BoardingRequest**:
   - Reservation vs actual occupancy
   - Asynchronous authorization with Grasida host
5. **Implement Manifest and AuthorizationEvidence**:
   - Vehicle explicit boarded person IDs
   - Authorization evidence for crossing
6. **Implement boarding protocol**:
   - Emit boarding request when person reaches transfer point
   - Host provides authorization evidence
   - Core rechecks and applies authorization
   - Update location/reservation on success
7. **Create people invariants fixtures**:
   - One location per person
   - Denied boarding leaves location unchanged
   - Occupancy and reservations obey capacity

## Phase 5: Core API, Snapshots and Hardening (A5 - Core API, snapshots and hardening)

### Goals:
- Restart/replay, versioned API, cached observations
- Scale profiles and failure scenarios

### Steps:
1. **Implement snapshot system**:
   - Periodic world state snapshots
   - Incremental snapshots for efficiency
   - Versioned format for backward compatibility
2. **Create API layer**:
   - Versioned command submission
   - State query endpoints (read-only projections)
   - Event stream for movement/events
3. **Implement restart/replay**:
   - Restore from snapshot + command log
   - Deterministic replay verification
4. **Add observability**:
   - Metrics for simulation performance
   - Health checks for world invariants
5. **Create hardening fixtures**:
   - Save/load cycle preservation
   - Interrupted external request handling
   - Scale testing with increased ships/people

## Phase 6: Frontend Integration (Optional but implied by "front on top of it")

### Goals:
- Provide frontend with ability to visualize and interact with simulation

### Steps:
1. **Define frontend API**:
   - Real-time state updates (WebSocket or polling)
   - Command submission interface
   - Event subscription for movements, boarding, etc.
2. **Choose frontend technology**:
   - Options: HTML/JavaScript, React, Vue, Svelte, etc.
   - Could compile Rust core to WebAssembly for browser execution
3. **Implement basic visualization**:
   - Tile-based map rendering
   - Vehicle and person icons
   - Station/dock representation
4. **Create interaction controls**:
   - Start/pause simulation
   - Speed control
   - Simple command dispatch (e.g., create journey)

## Success Criteria for Each Phase

### Phase 0:
- Reproducible C++ build running reference scenario
- Normalized state export format defined
- Ontology subset documented

### Phase 1:
- Rust core compiles and passes primitive fixtures
- ID system, tile accessors, PRNG behave identically to C++ reference
- Command queue and execution framework functional

### Phase 2:
- Cargo split/merge conserves total units
- Loading/unloading at stations works correctly
- Order sharing between vehicles functional

### Phase 3:
- Ship moves correctly along water path
- Arrival at docks triggers appropriate orders
- Trace comparison shows <1% divergence in key metrics (position, cargo) vs C++

### Phase 4:
- Person location invariant maintained
- Boarding process works with authorization
- Manifest matches actual boarded persons
- Denial handling leaves person state unchanged

### Phase 5:
- Snapshots enable perfect restart/replay
- API versioning allows backward compatibility
- System handles 100+ ships and 1000+ people without invariant violations

### Frontend:
- Real-time visualization matches simulation state
- User can submit simple commands (create journey)
- Event stream shows person movements and ship activities

## Dependencies and Tools

- Rust toolchain (stable)
- Cargo workspace
- Optional: Docker for consistent C++ build environment
- Testing: criterion for benchmarking, property-based testing with proptest
- Serialization: serde for snapshots
- Async: tokio for host communication (if needed)
- WebAssembly: wasm-pack for frontend compilation (if web-based)

## Risks and Mitigations

1. **Clock/PRNG differences**: 
   - Mitigation: Extract and test primitive clock/PRNG functions early
   
2. **State ownership complexity**:
   - Mitigation: Start with simple arena-based ownership, iterate
   
3. **People simulation complexity**:
   - Mitigation: Implement location state machine thoroughly with unit tests
   
4. **Performance bottlenecks**:
   - Mitigation: Profile early, optimize data structures for cache locality
   
5. **Frontend integration complexity**:
   - Mitigation: Define clean API boundary early, use WebAssembly if browser-based

## Formal Verification with Lean 4

To achieve mathematical certainty for critical invariants, we will use the Lean 4 theorem prover to verify key properties of the simulation engine.

### Verification Goals
- Tile index bounds checking and conversion correctness
- ID generation and validity properties
- Cargo conservation (split/merge operations preserve total units)
- Person location invariant (each person has exactly one location)
- Order list sharing consistency
- Movement pathfinding correctness

### Approach
1. **Abstract Specification**: Write Lean specifications for core data structures and algorithms
2. **Property Verification**: Prove key invariants and properties about these specifications
3. **Rust Integration**: Ensure Rust implementation matches verified specifications (via testing or potential extraction)
4. **Continuous Verification**: Include Lean proofs in CI to guarantee ongoing correctness

### Implementation Steps
1. Set up Lean 4 project with mathlib4 (created in `lean/` directory)
2. Develop verified theories for:
   - Tile indexing and map bounds (`TileIndex.lean`)
   - Strongly-typed ID systems
   - Cargo split/merge conservation
   - Person location state machine
   - Order list sharing properties
3. Create correspondence tests between Lean specifications and Rust implementation
4. Run Lean verification as part of CI pipeline

### Tools
- Lean 4 theorem prover
- Mathlib4 mathematical library
- Custom verification theories in `lean/src/`

## Next Immediate Actions

1. Create Rust workspace and core crates
2. Implement transport-types crate with ID patterns
3. Set up Lean 4 verification project (already initialized)
4. Set up CI for building and testing
5. Begin Phase 0: Build C++ reference and export traces