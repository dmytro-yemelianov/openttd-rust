# Core Data Model (`memory/data_model.md`)

## Identifiers (`crates/transport-types/src/id.rs`)
All IDs are newtypes with `pub const INVALID = Self(MAX)`:
- `CompanyID(pub u32)`
- `VehicleID(pub u32)`
- `StationID(pub u32)`
- `OrderListID(pub u32)`
- `OrderIndex(pub u16)`
- `CargoType(pub u16)`
- `CargoPacketID(pub u32)`
- `EngineID(pub u32)`
- `PersonID(pub u64)`
- `JourneyID(pub u64)`
- `AssignmentID(pub u64)`

## Fundamental Units (`crates/transport-types/src/unit.rs`)
- `TileIndex(pub u32)`: Map tile index (offset = `y * width + x`).
- `Coord { pub x: i32, pub y: i32 }`: 2D map coordinate.
- `Ticks(pub u64)`: Absolute simulation tick counter.
- `Money(pub i64)`: Currency amount (checked math required by safeguards).
- `Distance(pub u32)`: Manhattan distance between tiles.
- `CargoAmount(pub u32)`: Units of cargo carried or queued.
- `Payment(pub i64)`: Payment calculation result.

## Key Simulation Entities (`crates/transport-world/src/`)
- `Map { size: MapSize, tiles: Vec<Tile> }`
  - `MapSize { width: u16, height: u16 }`
  - `Tile { base: TileBase (16B), extension: TileExtension (4B) }`: Exactly 20 bytes total.
- `Company { id: CompanyID, name: String, money: Money, color: u32 }`
- `Vehicle`:
  - `id: VehicleID`, `company_id: CompanyID`, `engine_id: EngineID`
  - `kind: VehicleKind`, `position: TileIndex`, `sub_tile_position: (i16, i16)`
  - `velocity: i16`, `current_order: Option<OrderIndex>`, `orders: OrderListID`
  - `cargo: Vec<(CargoType, CargoAmount)>`
- `Station { id: StationID, company_id: CompanyID, name: String, tile: TileIndex, cargo: HashMap<CargoType, CargoAmount> }`

## Simulation Engine (`crates/transport-sim/src/lib.rs`)
- `World`:
  - `map: Map`, `companies: HashMap<CompanyID, Company>`, `vehicles: HashMap<VehicleID, Vehicle>`
  - `stations: HashMap<StationID, Station>`, `order_lists: HashMap<OrderListID, Vec<OrderType>>`
  - `tick: Ticks`, `current_company_id: CompanyID`
- `Command` Enum:
  - `CreateCompany { name, money, color }`
  - `BuildStation { company_id, name, tiles }`
  - `BuildVehicle { company_id, engine_id, tile }`
  - `ModifyOrders { company_id, vehicle_id, orders }`
  - `MoveVehicle { company_id, vehicle_id, target_tile }`
- `CommandResult`: `Success` or `Failure(String)` (to be typed in task 1).
