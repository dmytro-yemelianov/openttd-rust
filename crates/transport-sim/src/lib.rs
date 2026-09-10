//! # Transport simulation core
//!
//! This crate contains the simulation logic:
//! - World state (map, entities, definitions)
//! - Command processing
//! - Simulation ticks (movement, loading/unloading, economy, etc.)

pub mod drivers;
pub mod kernel;

pub use drivers::*;
pub use kernel::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use transport_types::enum_::{OrderType, VehicleState};
use transport_types::unit::{CargoAmount, Money, Ticks};
use transport_types::{
    CargoType, CompanyID, EngineID, OrderIndex, OrderListID, StationID, TileIndex, VehicleID,
};
use transport_world::map::{Map, MapSize};
use transport_world::{definitions::*, entities::*};

/// Command queue entry for the simulation agentic loop
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEntry {
    pub command: Command,
    pub source: String, // Who/what submitted this command
    pub priority: u8,   // Priority level (higher = processed first)
}

impl CommandEntry {
    pub fn new(command: Command, source: String, priority: u8) -> Self {
        Self {
            command,
            source,
            priority,
        }
    }
}

/// A command that has been submitted to the queue with a monotonic submission ID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub submission_id: u64,
    pub entry: CommandEntry,
}

/// A strongly-typed successful outcome of command processing
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CommandSuccess {
    CompanyCreated(CompanyID),
    StationBuilt(StationID),
    VehiclePurchased(VehicleID),
    OrderListCreated(OrderListID),
    OrderAdded,
    CargoLoaded {
        transferred: transport_types::unit::CargoAmount,
    },
    CargoUnloaded {
        transferred: transport_types::unit::CargoAmount,
        revenue: Money,
    },
    Done,
}

/// A strongly-typed failure outcome of command processing
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CommandFailure {
    Unsupported(String),
    CompanyIdExhausted,
    StationIdExhausted,
    VehicleIdExhausted,
    OrderListIdExhausted,
    CompanyNotFound(CompanyID),
    StationNotFound(StationID),
    VehicleNotFound(VehicleID),
    OrderListNotFound(OrderListID),
    InsufficientFunds { required: Money, available: Money },
    OutOfBounds(TileIndex),
    TileOccupied(TileIndex),
    InvalidPlacement(String),
    CapacityExceeded,
    NoCargoAvailable,
}

/// The result of processing a command
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CommandResult {
    Success(CommandSuccess),
    Failure(CommandFailure),
}

impl CommandResult {
    pub fn is_success(&self) -> bool {
        matches!(self, CommandResult::Success(_))
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, CommandResult::Failure(_))
    }
}

/// Errors that can occur when enqueuing a command
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum EnqueueError {
    SequenceExhausted,
}

/// Error indicating that company ID generation has exhausted valid IDs
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum CompanyIdExhausted {
    Exhausted,
}

/// The stored outcome of processing a queued command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub result: CommandResult,
    #[serde(default)]
    pub execution_order: u64,
    pub tick: Ticks,
}

/// The main world state of the simulation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub map: Map,
    pub companies: HashMap<CompanyID, Company>,
    pub vehicles: HashMap<VehicleID, Vehicle>,
    pub stations: HashMap<StationID, Station>,
    pub engines: HashMap<EngineID, Engine>,
    pub cargo_types: HashMap<CargoType, Cargo>,
    pub order_lists: HashMap<OrderListID, Vec<OrderType>>, // Simplified: list of order types
    pub current_company_id: CompanyID,
    pub tick: Ticks,
    pub money_scale: u32, // For inflation, etc.
    /// Derived spatial lookup mapping tile indices to station IDs
    #[serde(skip)]
    pub tile_to_station: HashMap<TileIndex, StationID>,
    // TODO: more fields (economy, climate, etc.)
}

impl World {
    pub fn new(map_size: MapSize) -> Self {
        let mut engines = HashMap::new();
        engines.insert(
            EngineID(1),
            Engine::new(
                EngineID(1),
                "Coastal Cargo Vessel".to_string(),
                transport_types::enum_::VehicleKind::Ship,
                200,
                100,
                50,
                transport_types::unit::Speed(10.0),
                Money(5000), // cost
                Money(20),   // running cost
                vec![
                    transport_types::enum_::CargoClass::Bulk,
                    transport_types::enum_::CargoClass::Passenger,
                ],
                1930,
                None,
            ),
        );
        let mut cargo_types = HashMap::new();
        cargo_types.insert(
            CargoType(1),
            Cargo::new(
                CargoType(1),
                "Grain".to_string(),
                transport_types::enum_::CargoClass::Bulk,
            ),
        );

        Self {
            map: Map::new(map_size),
            companies: HashMap::new(),
            vehicles: HashMap::new(),
            stations: HashMap::new(),
            engines,
            cargo_types,
            order_lists: HashMap::new(),
            current_company_id: CompanyID::INVALID, // Will be set when first company is created
            tick: Ticks(0),
            money_scale: 100, // 1.00 scale
            tile_to_station: HashMap::new(),
        }
    }

    /// Get a company by ID
    pub fn get_company(&self, id: &CompanyID) -> Option<&Company> {
        self.companies.get(id)
    }

    /// Get a mutable company by ID
    pub fn get_company_mut(&mut self, id: &CompanyID) -> Option<&mut Company> {
        self.companies.get_mut(id)
    }

    /// Advance the tick counter (called by the simulator's agentic loop)
    pub fn advance_tick(&mut self) {
        self.tick.0 += 1;
    }

    /// Get a station by ID
    pub fn get_station(&self, id: &StationID) -> Option<&Station> {
        self.stations.get(id)
    }

    /// Rebuild derived spatial index mapping tiles to stations
    pub fn rebuild_derived_indexes(&mut self) {
        self.tile_to_station.clear();
        for (station_id, station) in &self.stations {
            for tile in &station.tiles {
                self.tile_to_station.insert(*tile, *station_id);
            }
        }
    }

    /// Get station at specific tile (using derived spatial lookup)
    pub fn get_station_at_tile(&self, tile: &TileIndex) -> Option<&Station> {
        self.tile_to_station
            .get(tile)
            .and_then(|id| self.stations.get(id))
    }
}

/// Commands that can be issued to the simulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Create a new company
    CreateCompany {
        name: String,
        money: Money,
        color: u32,
    },
    /// Build a station (dock)
    BuildStation {
        company_id: CompanyID,
        name: String,
        tiles: Vec<TileIndex>,
    },
    /// Purchase a vehicle
    PurchaseVehicle {
        company_id: CompanyID,
        engine_id: EngineID,
        kind: transport_types::enum_::VehicleKind,
        position: TileIndex,
        order_list_id: OrderListID,
    },
    /// Create an order list
    CreateOrderList {},
    /// Add an order to an order list
    AddOrderToList {
        order_list_id: OrderListID,
        order: OrderType,
    },
    /// Load cargo at a station
    LoadCargo {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: transport_types::unit::CargoAmount,
    },
    /// Unload cargo at a station
    UnloadCargo {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: transport_types::unit::CargoAmount,
    }, // TODO: more commands (delete, modify, etc.)
}

impl Command {
    /// Returns the static variant name of the command
    pub fn name(&self) -> &'static str {
        match self {
            Command::CreateCompany { .. } => "CreateCompany",
            Command::BuildStation { .. } => "BuildStation",
            Command::PurchaseVehicle { .. } => "PurchaseVehicle",
            Command::CreateOrderList { .. } => "CreateOrderList",
            Command::AddOrderToList { .. } => "AddOrderToList",
            Command::LoadCargo { .. } => "LoadCargo",
            Command::UnloadCargo { .. } => "UnloadCargo",
        }
    }
}

/// The simulator that processes commands and advances the world
/// This is the "agentic loop" - the main iteration loop of the simulation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Simulator {
    pub world: World,
    /// Command queue for the agentic loop (FIFO for equal priorities, priority-sorted)
    pub command_queue: VecDeque<QueuedCommand>,
    /// Next submission ID to assign to queued commands
    #[serde(default)]
    pub next_submission_id: u64,
    /// Outcomes of processed queued commands, keyed by submission ID
    #[serde(default)]
    pub outcomes: HashMap<u64, CommandOutcome>,
    /// Next execution order number to assign
    #[serde(default)]
    pub execution_order_next: u64,
    /// Pre-allocated reusable buffer for deterministic vehicle iteration (zero per-tick allocations)
    #[serde(skip)]
    pub vehicle_order_buffer: Vec<VehicleID>,
}

impl Simulator {
    pub fn new(map_size: MapSize) -> Self {
        Self {
            world: World::new(map_size),
            command_queue: VecDeque::new(),
            next_submission_id: 0,
            outcomes: HashMap::new(),
            execution_order_next: 0,
            vehicle_order_buffer: Vec::new(),
        }
    }

    /// Add a command to the queue with a monotonically increasing submission ID.
    /// Commands are inserted in priority order (higher priority first).
    /// Equal-priority commands maintain strict FIFO arrival order.
    /// Returns the assigned submission ID or an error if sequence is exhausted.
    pub fn enqueue_command(
        &mut self,
        command: Command,
        source: String,
        priority: u8,
    ) -> Result<u64, EnqueueError> {
        let submission_id = self.next_submission_id;
        self.next_submission_id = self
            .next_submission_id
            .checked_add(1)
            .ok_or(EnqueueError::SequenceExhausted)?;

        let queued_cmd = QueuedCommand {
            submission_id,
            entry: CommandEntry::new(command, source, priority),
        };

        // Strict FIFO for equal priority: find first element with lower priority
        let insert_pos = self
            .command_queue
            .iter()
            .position(|e| e.entry.priority < priority)
            .unwrap_or(self.command_queue.len());

        self.command_queue.insert(insert_pos, queued_cmd);
        Ok(submission_id)
    }

    /// Process a command immediately using the authoritative unified handler.
    pub fn process_command(&mut self, command: Command) -> CommandResult {
        self.process_command_unified(&command)
    }

    /// Drain outcomes of processed queued commands.
    /// Returns an iterator yielding (submission_id, outcome) pairs and removes them from the store.
    pub fn drain_outcomes(&mut self) -> impl Iterator<Item = (u64, CommandOutcome)> + '_ {
        self.outcomes.drain()
    }

    /// Core agentic loop: advance the simulation by one tick
    ///
    /// This is the main iteration loop:
    /// 1. Process pending commands from the queue
    /// 2. Advance all vehicles according to their orders
    /// 3. Handle cargo loading/unloading at stations
    /// 4. Process person boarding/alighting
    /// 5. Emit simulation events for observability
    /// 6. Advance the tick counter
    pub fn tick(&mut self) {
        // Step 1: Process pending commands from the queue using unified handler
        while let Some(queued_cmd) = self.command_queue.pop_front() {
            let result = self.process_command_unified(&queued_cmd.entry.command);
            self.outcomes.insert(
                queued_cmd.submission_id,
                CommandOutcome {
                    result,
                    execution_order: self.execution_order_next,
                    tick: self.world.tick,
                },
            );
            self.execution_order_next = self
                .execution_order_next
                .checked_add(1)
                .expect("Execution order sequence exhausted");
        }

        // Step 2: Advance all vehicles according to their orders (deterministic order, zero-alloc buffer)
        self.vehicle_order_buffer.clear();
        self.vehicle_order_buffer
            .extend(self.world.vehicles.keys().copied());
        self.vehicle_order_buffer.sort_unstable();
        for i in 0..self.vehicle_order_buffer.len() {
            let vehicle_id = self.vehicle_order_buffer[i];
            self.advance_vehicle_orders(vehicle_id);
        }

        // Step 3: Handle cargo loading/unloading at stations (reuses sorted vehicle buffer)
        if let Err(e) = self.handle_cargo_transfer() {
            eprintln!("Cargo transfer error: {e:?}");
        }

        // Step 4: Process person boarding/alighting
        if let Err(e) = self.process_person_transport() {
            eprintln!("Person transport error: {e:?}");
        }

        // Step 5: Advance tick counter
        self.world.advance_tick();
    }

    /// Unified command handler for both immediate and queued commands.
    /// Unsupported commands and failed commands mutate ZERO world state.
    pub fn process_command_unified(&mut self, command: &Command) -> CommandResult {
        match command {
            Command::CreateCompany { name, money, color } => match self.next_company_id_checked() {
                Ok(id) => {
                    let company = Company::new(id, name.clone(), *money, *color);
                    self.world.companies.insert(id, company);
                    if self.world.current_company_id == CompanyID::INVALID {
                        self.world.current_company_id = id;
                    }
                    CommandResult::Success(CommandSuccess::CompanyCreated(id))
                }
                Err(CompanyIdExhausted::Exhausted) => {
                    CommandResult::Failure(CommandFailure::CompanyIdExhausted)
                }
            },
            Command::BuildStation {
                company_id,
                name,
                tiles,
            } => {
                if !self.world.companies.contains_key(company_id) {
                    return CommandResult::Failure(CommandFailure::CompanyNotFound(*company_id));
                }
                if tiles.is_empty() {
                    return CommandResult::Failure(CommandFailure::InvalidPlacement(
                        "Station must contain at least one tile".to_string(),
                    ));
                }
                for t in tiles {
                    if !self.world.map.size().is_valid_index(*t) {
                        return CommandResult::Failure(CommandFailure::OutOfBounds(*t));
                    }
                    if self.world.tile_to_station.contains_key(t)
                        || self.world.map.station_at(*t).is_some()
                    {
                        return CommandResult::Failure(CommandFailure::TileOccupied(*t));
                    }
                }
                let cost = Money(1000 * tiles.len() as i64);
                let company = self.world.companies.get_mut(company_id).unwrap();
                if company.money.0 < cost.0 {
                    return CommandResult::Failure(CommandFailure::InsufficientFunds {
                        required: cost,
                        available: company.money,
                    });
                }
                company.money = match company.money.sub_checked(cost) {
                    Ok(m) => m,
                    Err(_) => {
                        return CommandResult::Failure(CommandFailure::InsufficientFunds {
                            required: cost,
                            available: company.money,
                        })
                    }
                };
                let station_id = match self.next_station_id_checked() {
                    Ok(id) => id,
                    Err(f) => return CommandResult::Failure(f),
                };
                let mut station = Station::new(station_id, name.clone(), tiles.clone());
                station.company_id = Some(*company_id);
                station.operator_rating = 100;
                for cargo_id in self.world.cargo_types.keys() {
                    station.goods.push(GoodsEntry::new(*cargo_id));
                }
                for t in tiles {
                    let _ = self
                        .world
                        .map
                        .set_station_at(*t, station_id, Some(*company_id));
                    self.world.tile_to_station.insert(*t, station_id);
                }
                self.world.stations.insert(station_id, station);
                CommandResult::Success(CommandSuccess::StationBuilt(station_id))
            }
            Command::PurchaseVehicle {
                company_id,
                engine_id,
                kind,
                position,
                order_list_id,
            } => {
                if !self.world.companies.contains_key(company_id) {
                    return CommandResult::Failure(CommandFailure::CompanyNotFound(*company_id));
                }
                if !self.world.order_lists.contains_key(order_list_id) {
                    return CommandResult::Failure(CommandFailure::OrderListNotFound(
                        *order_list_id,
                    ));
                }
                if !self.world.map.size().is_valid_index(*position) {
                    return CommandResult::Failure(CommandFailure::OutOfBounds(*position));
                }
                let cost = self
                    .world
                    .engines
                    .get(engine_id)
                    .map(|e| e.cost)
                    .unwrap_or(Money(5000));
                let company = self.world.companies.get_mut(company_id).unwrap();
                if company.money.0 < cost.0 {
                    return CommandResult::Failure(CommandFailure::InsufficientFunds {
                        required: cost,
                        available: company.money,
                    });
                }
                company.money = match company.money.sub_checked(cost) {
                    Ok(m) => m,
                    Err(_) => {
                        return CommandResult::Failure(CommandFailure::InsufficientFunds {
                            required: cost,
                            available: company.money,
                        })
                    }
                };
                let vehicle_id = match self.next_vehicle_id_checked() {
                    Ok(id) => id,
                    Err(f) => return CommandResult::Failure(f),
                };
                let vehicle = Vehicle::new(
                    vehicle_id,
                    *company_id,
                    *engine_id,
                    *kind,
                    *position,
                    *order_list_id,
                );
                self.world.vehicles.insert(vehicle_id, vehicle);
                CommandResult::Success(CommandSuccess::VehiclePurchased(vehicle_id))
            }
            Command::CreateOrderList {} => {
                let id = match self.next_order_list_id_checked() {
                    Ok(id) => id,
                    Err(f) => return CommandResult::Failure(f),
                };
                self.world.order_lists.insert(id, Vec::new());
                CommandResult::Success(CommandSuccess::OrderListCreated(id))
            }
            Command::AddOrderToList {
                order_list_id,
                order,
            } => {
                let list = match self.world.order_lists.get_mut(order_list_id) {
                    Some(l) => l,
                    None => {
                        return CommandResult::Failure(CommandFailure::OrderListNotFound(
                            *order_list_id,
                        ))
                    }
                };
                list.push(order.clone());
                CommandResult::Success(CommandSuccess::OrderAdded)
            }
            Command::LoadCargo {
                vehicle_id,
                station_id,
                cargo_type,
                amount,
            } => {
                let vehicle = match self.world.vehicles.get_mut(vehicle_id) {
                    Some(v) => v,
                    None => {
                        return CommandResult::Failure(CommandFailure::VehicleNotFound(*vehicle_id))
                    }
                };
                let station = match self.world.stations.get_mut(station_id) {
                    Some(s) => s,
                    None => {
                        return CommandResult::Failure(CommandFailure::StationNotFound(*station_id))
                    }
                };
                if !station.tiles.contains(&vehicle.position) {
                    return CommandResult::Failure(CommandFailure::InvalidPlacement(
                        "Vehicle is not positioned at station".to_string(),
                    ));
                }
                let goods = match station
                    .goods
                    .iter_mut()
                    .find(|g| g.cargo_type == *cargo_type)
                {
                    Some(g) => g,
                    None => return CommandResult::Failure(CommandFailure::NoCargoAvailable),
                };
                let transfer = std::cmp::min(goods.amount.0, amount.0);
                if transfer == 0 {
                    return CommandResult::Failure(CommandFailure::NoCargoAvailable);
                }
                goods.amount.0 -= transfer;
                if let Some(entry) = vehicle.cargo.iter_mut().find(|(c, _)| *c == *cargo_type) {
                    entry.1 .0 += transfer;
                } else {
                    vehicle.cargo.push((*cargo_type, CargoAmount(transfer)));
                }
                CommandResult::Success(CommandSuccess::CargoLoaded {
                    transferred: CargoAmount(transfer),
                })
            }
            Command::UnloadCargo {
                vehicle_id,
                station_id,
                cargo_type,
                amount,
            } => {
                let vehicle = match self.world.vehicles.get_mut(vehicle_id) {
                    Some(v) => v,
                    None => {
                        return CommandResult::Failure(CommandFailure::VehicleNotFound(*vehicle_id))
                    }
                };
                let station = match self.world.stations.get_mut(station_id) {
                    Some(s) => s,
                    None => {
                        return CommandResult::Failure(CommandFailure::StationNotFound(*station_id))
                    }
                };
                if !station.tiles.contains(&vehicle.position) {
                    return CommandResult::Failure(CommandFailure::InvalidPlacement(
                        "Vehicle is not positioned at station".to_string(),
                    ));
                }
                let cargo_entry = match vehicle.cargo.iter_mut().find(|(c, _)| *c == *cargo_type) {
                    Some(e) => e,
                    None => return CommandResult::Failure(CommandFailure::NoCargoAvailable),
                };
                let transfer = std::cmp::min(cargo_entry.1 .0, amount.0);
                if transfer == 0 {
                    return CommandResult::Failure(CommandFailure::NoCargoAvailable);
                }
                cargo_entry.1 .0 -= transfer;
                let company_id = vehicle.company_id;
                if let Some(g) = station
                    .goods
                    .iter_mut()
                    .find(|g| g.cargo_type == *cargo_type)
                {
                    g.delivered_since_last_visit.0 += transfer;
                } else {
                    let mut g = GoodsEntry::new(*cargo_type);
                    g.delivered_since_last_visit = CargoAmount(transfer);
                    station.goods.push(g);
                }
                let revenue = Money(transfer as i64 * 10);
                if let Some(company) = self.world.companies.get_mut(&company_id) {
                    company.money = company.money.saturating_add(revenue);
                }
                CommandResult::Success(CommandSuccess::CargoUnloaded {
                    transferred: CargoAmount(transfer),
                    revenue,
                })
            }
        }
    }

    /// Generate the next CompanyID using checked arithmetic.
    /// Never returns CompanyID::INVALID and never overwrites an existing company.
    pub fn next_company_id_checked(&self) -> Result<CompanyID, CompanyIdExhausted> {
        let max_id = self.world.companies.keys().map(|id| id.0).max();
        let next_val = match max_id {
            None => 0,
            Some(v) => v.checked_add(1).ok_or(CompanyIdExhausted::Exhausted)?,
        };
        if next_val == CompanyID::INVALID.0 {
            return Err(CompanyIdExhausted::Exhausted);
        }
        Ok(CompanyID(next_val))
    }

    /// Generate the next StationID using checked arithmetic.
    pub fn next_station_id_checked(&self) -> Result<StationID, CommandFailure> {
        let max_id = self.world.stations.keys().map(|id| id.0).max();
        let next_val = match max_id {
            None => 0,
            Some(v) => v.checked_add(1).ok_or(CommandFailure::StationIdExhausted)?,
        };
        if next_val == StationID::INVALID.0 {
            return Err(CommandFailure::StationIdExhausted);
        }
        Ok(StationID(next_val))
    }

    /// Generate the next VehicleID using checked arithmetic.
    pub fn next_vehicle_id_checked(&self) -> Result<VehicleID, CommandFailure> {
        let max_id = self.world.vehicles.keys().map(|id| id.0).max();
        let next_val = match max_id {
            None => 0,
            Some(v) => v.checked_add(1).ok_or(CommandFailure::VehicleIdExhausted)?,
        };
        if next_val == VehicleID::INVALID.0 {
            return Err(CommandFailure::VehicleIdExhausted);
        }
        Ok(VehicleID(next_val))
    }

    /// Generate the next OrderListID using checked arithmetic.
    pub fn next_order_list_id_checked(&self) -> Result<OrderListID, CommandFailure> {
        let max_id = self.world.order_lists.keys().map(|id| id.0).max();
        let next_val = match max_id {
            None => 0,
            Some(v) => v
                .checked_add(1)
                .ok_or(CommandFailure::OrderListIdExhausted)?,
        };
        if next_val == OrderListID::INVALID.0 {
            return Err(CommandFailure::OrderListIdExhausted);
        }
        Ok(OrderListID(next_val))
    }

    #[deprecated(note = "Use next_company_id_checked instead to avoid ID exhaustion issues")]
    pub fn next_company_id(&mut self) -> CompanyID {
        self.next_company_id_checked().unwrap_or(CompanyID(0))
    }

    /// Advance vehicle order progression deterministically without arbitrary thresholds.
    fn advance_vehicle_orders(&mut self, vehicle_id: VehicleID) {
        let vehicle = match self.world.vehicles.get_mut(&vehicle_id) {
            Some(v) => v,
            None => return,
        };

        let orders = match self.world.order_lists.get(&vehicle.orders) {
            Some(list) if !list.is_empty() => list,
            _ => {
                // Empty schedule or invalid list reference: vehicle stays safely idle
                vehicle.current_order = None;
                vehicle.state = VehicleState::Idle;
                return;
            }
        };

        let current_idx = match vehicle.current_order {
            Some(idx) if (idx.0 as usize) < orders.len() => idx.0 as usize,
            _ => {
                // Initialize to the first order
                vehicle.current_order = Some(OrderIndex(0));
                vehicle.state = VehicleState::Idle;
                0
            }
        };

        match &orders[current_idx] {
            OrderType::WaitTime(ticks) => {
                match vehicle.state {
                    VehicleState::Waiting(rem) => {
                        if rem <= 1 {
                            // Waiting completed -> advance to next order (with wrap-around)
                            let next_idx = (current_idx + 1) % orders.len();
                            vehicle.current_order = Some(OrderIndex(next_idx as u16));
                            vehicle.state = VehicleState::Idle;
                        } else {
                            vehicle.state = VehicleState::Waiting(rem - 1);
                        }
                    }
                    _ => {
                        if *ticks <= 1 {
                            // Completed immediately on this tick
                            let next_idx = (current_idx + 1) % orders.len();
                            vehicle.current_order = Some(OrderIndex(next_idx as u16));
                            vehicle.state = VehicleState::Idle;
                        } else {
                            vehicle.state = VehicleState::Waiting(*ticks - 1);
                        }
                    }
                }
            }
            OrderType::GoToStation { station_id, .. } => {
                let station_opt = self.world.stations.get(station_id);
                let (is_at_station, target_tile) = match station_opt {
                    Some(s) => (
                        s.tiles.contains(&vehicle.position),
                        s.tiles.first().copied(),
                    ),
                    None => {
                        vehicle.state = VehicleState::Idle;
                        return;
                    }
                };

                if is_at_station {
                    let next_idx = (current_idx + 1) % orders.len();
                    vehicle.current_order = Some(OrderIndex(next_idx as u16));
                    vehicle.state = VehicleState::Loading;
                } else if let Some(target) = target_tile {
                    vehicle.state = VehicleState::Traveling;
                    let next_pos = step_towards(vehicle.position, target, &self.world.map);
                    vehicle.position = next_pos;
                    let arrived = self
                        .world
                        .stations
                        .get(station_id)
                        .map(|s| s.tiles.contains(&vehicle.position))
                        .unwrap_or(false);
                    if arrived {
                        let next_idx = (current_idx + 1) % orders.len();
                        vehicle.current_order = Some(OrderIndex(next_idx as u16));
                        vehicle.state = VehicleState::Loading;
                    }
                }
            }
            OrderType::GoToTile { x, y } => {
                let target = TileIndex::new(*x as u16, *y as u16);
                if vehicle.position == target {
                    let next_idx = (current_idx + 1) % orders.len();
                    vehicle.current_order = Some(OrderIndex(next_idx as u16));
                    vehicle.state = VehicleState::Idle;
                } else {
                    vehicle.state = VehicleState::Traveling;
                    let next_pos = step_towards(vehicle.position, target, &self.world.map);
                    vehicle.position = next_pos;
                    if vehicle.position == target {
                        let next_idx = (current_idx + 1) % orders.len();
                        vehicle.current_order = Some(OrderIndex(next_idx as u16));
                        vehicle.state = VehicleState::Idle;
                    }
                }
            }
            OrderType::NoOrder => {
                vehicle.current_order = None;
                vehicle.state = VehicleState::Idle;
            }
            _ => {
                // Remaining order types
            }
        }
    }

    /// Handle cargo loading/unloading between vehicles and stations (deterministic order, zero-alloc)
    fn handle_cargo_transfer(&mut self) -> Result<(), String> {
        for i in 0..self.vehicle_order_buffer.len() {
            let vehicle_id = self.vehicle_order_buffer[i];
            let (is_loading, pos, company_id) = match self.world.vehicles.get(&vehicle_id) {
                Some(v) => (v.state == VehicleState::Loading, v.position, v.company_id),
                None => continue,
            };
            if !is_loading {
                continue;
            }

            let station_id = match self.world.get_station_at_tile(&pos) {
                Some(s) => s.id,
                None => continue,
            };

            // 1. Unload any cargo on the vehicle
            let mut unloaded = Vec::new();
            if let Some(vehicle) = self.world.vehicles.get_mut(&vehicle_id) {
                for (cargo_type, amount) in vehicle.cargo.drain(..) {
                    if amount.0 > 0 {
                        unloaded.push((cargo_type, amount));
                    }
                }
            }
            for (cargo_type, amount) in unloaded {
                if let Some(station) = self.world.stations.get_mut(&station_id) {
                    if let Some(g) = station
                        .goods
                        .iter_mut()
                        .find(|g| g.cargo_type == cargo_type)
                    {
                        g.delivered_since_last_visit.0 += amount.0;
                    } else {
                        let mut g = GoodsEntry::new(cargo_type);
                        g.delivered_since_last_visit = amount;
                        station.goods.push(g);
                    }
                }
                // Revenue: 10 per cargo unit delivered
                let revenue = Money(amount.0 as i64 * 10);
                if let Some(company) = self.world.companies.get_mut(&company_id) {
                    company.money = company.money.saturating_add(revenue);
                }
            }

            // 2. Load available cargo from station into vehicle (up to capacity)
            let capacity = 50u32;
            let current_load: u32 = self
                .world
                .vehicles
                .get(&vehicle_id)
                .map(|v| v.cargo.iter().map(|(_, a)| a.0).sum())
                .unwrap_or(0);
            let available_cap = capacity.saturating_sub(current_load);
            if available_cap > 0 {
                if let Some(station) = self.world.stations.get_mut(&station_id) {
                    for goods in &mut station.goods {
                        if goods.amount.0 > 0 {
                            let transfer = std::cmp::min(goods.amount.0, available_cap);
                            goods.amount.0 -= transfer;
                            if let Some(vehicle) = self.world.vehicles.get_mut(&vehicle_id) {
                                if let Some(e) = vehicle
                                    .cargo
                                    .iter_mut()
                                    .find(|(c, _)| *c == goods.cargo_type)
                                {
                                    e.1 .0 += transfer;
                                } else {
                                    vehicle
                                        .cargo
                                        .push((goods.cargo_type, CargoAmount(transfer)));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Process person boarding/alighting from vehicles
    fn process_person_transport(&mut self) -> Result<(), String> {
        // For each person aboard a vehicle, check their journey progress
        // TODO: Implement person transport logic
        Ok(())
    }
}

/// Discrete single-tile step towards target coordinate within map bounds
pub fn step_towards(from: TileIndex, to: TileIndex, map: &Map) -> TileIndex {
    let dx = (to.x as i32) - (from.x as i32);
    let dy = (to.y as i32) - (from.y as i32);
    if dx != 0 {
        let next_x = if dx > 0 { from.x + 1 } else { from.x - 1 };
        let cand = TileIndex::new(next_x, from.y);
        if map.size().is_valid_index(cand) {
            return cand;
        }
    }
    if dy != 0 {
        let next_y = if dy > 0 { from.y + 1 } else { from.y - 1 };
        let cand = TileIndex::new(from.x, next_y);
        if map.size().is_valid_index(cand) {
            return cand;
        }
    }
    from
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failed_command_leaves_world_unmutated() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let failed_cmd = Command::BuildStation {
            company_id: CompanyID(99), // non-existent company
            name: "Test Dock".to_string(),
            tiles: vec![TileIndex::new(0, 0)],
        };

        // Immediate path
        let result = sim.process_command(failed_cmd.clone());
        assert_eq!(
            result,
            CommandResult::Failure(CommandFailure::CompanyNotFound(CompanyID(99)))
        );
        assert_eq!(sim.world.companies.len(), 0);
        assert_eq!(sim.world.stations.len(), 0);

        // Queued path
        let sub_id = sim
            .enqueue_command(failed_cmd, "test".to_string(), 1)
            .unwrap();
        sim.tick();

        let outcomes: Vec<_> = sim.drain_outcomes().collect();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].0, sub_id);
        assert_eq!(
            outcomes[0].1.result,
            CommandResult::Failure(CommandFailure::CompanyNotFound(CompanyID(99)))
        );
        assert_eq!(sim.world.companies.len(), 0);
        assert_eq!(sim.world.stations.len(), 0);
    }

    #[test]
    fn test_immediate_and_queued_equivalence() {
        let mut sim_imm = Simulator::new(MapSize::new(10, 10));
        let mut sim_queued = Simulator::new(MapSize::new(10, 10));

        let cmd = Command::CreateCompany {
            name: "Transport Co".to_string(),
            money: Money(50000),
            color: 0x00FF00,
        };

        let imm_res = sim_imm.process_command(cmd.clone());
        let sub_id = sim_queued
            .enqueue_command(cmd, "test".to_string(), 5)
            .unwrap();
        sim_queued.tick();

        let outcomes: Vec<_> = sim_queued.drain_outcomes().collect();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].0, sub_id);
        assert_eq!(imm_res, outcomes[0].1.result);

        assert_eq!(sim_imm.world.companies.len(), 1);
        assert_eq!(sim_queued.world.companies.len(), 1);
        let id_imm = sim_imm.world.current_company_id;
        let id_queued = sim_queued.world.current_company_id;
        assert_eq!(id_imm, id_queued);
        assert_eq!(
            sim_imm.world.get_company(&id_imm),
            sim_queued.world.get_company(&id_queued)
        );
    }

    #[test]
    fn test_created_company_id_returned() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let cmd = Command::CreateCompany {
            name: "First Co".to_string(),
            money: Money(10000),
            color: 0xFF0000,
        };

        let res = sim.process_command(cmd);
        match res {
            CommandResult::Success(CommandSuccess::CompanyCreated(id)) => {
                assert_eq!(id, CompanyID(0));
                assert_eq!(sim.world.companies.get(&id).unwrap().name, "First Co");
            }
            other => panic!("Expected CompanyCreated, got {other:?}"),
        }
    }

    #[test]
    fn test_priority_and_fifo_order() {
        let mut sim = Simulator::new(MapSize::new(10, 10));

        // Submit low priority, high priority, and equal priority
        let id_low = sim
            .enqueue_command(
                Command::CreateCompany {
                    name: "Low".to_string(),
                    money: Money(1),
                    color: 1,
                },
                "test".to_string(),
                1,
            )
            .unwrap();

        let id_high = sim
            .enqueue_command(
                Command::CreateCompany {
                    name: "High".to_string(),
                    money: Money(2),
                    color: 2,
                },
                "test".to_string(),
                10,
            )
            .unwrap();

        let id_high_second = sim
            .enqueue_command(
                Command::CreateCompany {
                    name: "High2".to_string(),
                    money: Money(3),
                    color: 3,
                },
                "test".to_string(),
                10,
            )
            .unwrap();

        assert_eq!(sim.command_queue[0].submission_id, id_high);
        assert_eq!(sim.command_queue[1].submission_id, id_high_second);
        assert_eq!(sim.command_queue[2].submission_id, id_low);

        sim.tick();

        let mut outcomes: Vec<_> = sim.drain_outcomes().collect();
        outcomes.sort_by_key(|o| o.1.execution_order);

        assert_eq!(outcomes[0].0, id_high);
        assert_eq!(outcomes[0].1.execution_order, 0);

        assert_eq!(outcomes[1].0, id_high_second);
        assert_eq!(outcomes[1].1.execution_order, 1);

        assert_eq!(outcomes[2].0, id_low);
        assert_eq!(outcomes[2].1.execution_order, 2);
    }

    #[test]
    fn test_outcomes_drain_exactly_once() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        sim.enqueue_command(
            Command::CreateCompany {
                name: "DrainTest".to_string(),
                money: Money(100),
                color: 1,
            },
            "test".to_string(),
            1,
        )
        .unwrap();

        sim.tick();
        let first_drain: Vec<_> = sim.drain_outcomes().collect();
        assert_eq!(first_drain.len(), 1);

        let second_drain: Vec<_> = sim.drain_outcomes().collect();
        assert_eq!(second_drain.len(), 0);
    }

    #[test]
    fn test_company_id_exhaustion_handled() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        // Force world to near exhaustion
        let near_max_id = CompanyID(CompanyID::INVALID.0 - 1);
        sim.world.companies.insert(
            near_max_id,
            Company::new(near_max_id, "MaxCo".to_string(), Money(1), 1),
        );

        let res = sim.process_command(Command::CreateCompany {
            name: "OverflowCo".to_string(),
            money: Money(1),
            color: 2,
        });

        assert_eq!(
            res,
            CommandResult::Failure(CommandFailure::CompanyIdExhausted)
        );
        assert_eq!(sim.world.companies.len(), 1);
    }

    #[test]
    fn test_empty_order_list_stays_idle() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let list_id = OrderListID(1);
        sim.world.order_lists.insert(list_id, Vec::new());

        let vehicle_id = VehicleID(1);
        let vehicle = Vehicle::new(
            vehicle_id,
            CompanyID(0),
            EngineID(0),
            transport_types::enum_::VehicleKind::Ship,
            TileIndex::new(0, 0),
            list_id,
        );
        sim.world.vehicles.insert(vehicle_id, vehicle);

        sim.tick();

        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, None);
        assert_eq!(v.state, VehicleState::Idle);
    }

    #[test]
    fn test_wait_time_order_progression() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let list_id = OrderListID(1);
        sim.world
            .order_lists
            .insert(list_id, vec![OrderType::WaitTime(3), OrderType::NoOrder]);

        let vehicle_id = VehicleID(1);
        let vehicle = Vehicle::new(
            vehicle_id,
            CompanyID(0),
            EngineID(0),
            transport_types::enum_::VehicleKind::Ship,
            TileIndex::new(0, 0),
            list_id,
        );
        sim.world.vehicles.insert(vehicle_id, vehicle);

        // Tick 1: enters wait state
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, Some(OrderIndex(0)));
        assert_eq!(v.state, VehicleState::Waiting(2));

        // Tick 2: continues wait
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, Some(OrderIndex(0)));
        assert_eq!(v.state, VehicleState::Waiting(1));

        // Tick 3: wait finishes -> advances to order 1 (NoOrder)
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, Some(OrderIndex(1)));

        // Tick 4: executes NoOrder -> becomes Idle with current_order None
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, None);
        assert_eq!(v.state, VehicleState::Idle);
    }

    #[test]
    fn test_orders_with_ids_above_999() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        // OrderListID well above legacy 999 threshold
        let list_id = OrderListID(5000);
        sim.world.order_lists.insert(
            list_id,
            vec![OrderType::WaitTime(1), OrderType::WaitTime(1)],
        );

        let vehicle_id = VehicleID(10000);
        let vehicle = Vehicle::new(
            vehicle_id,
            CompanyID(0),
            EngineID(0),
            transport_types::enum_::VehicleKind::Ship,
            TileIndex::new(0, 0),
            list_id,
        );
        sim.world.vehicles.insert(vehicle_id, vehicle);

        // Tick 1: completes 1-tick wait and transitions to order 1
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, Some(OrderIndex(1)));

        // Tick 2: completes second 1-tick wait and wraps to order 0
        sim.tick();
        let v = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(v.current_order, Some(OrderIndex(0)));
    }

    #[test]
    fn test_deterministic_vehicle_processing_order() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let list_id = OrderListID(1);
        sim.world
            .order_lists
            .insert(list_id, vec![OrderType::WaitTime(10)]);

        // Insert in non-sorted order
        for id in [VehicleID(40), VehicleID(2), VehicleID(15), VehicleID(1)] {
            sim.world.vehicles.insert(
                id,
                Vehicle::new(
                    id,
                    CompanyID(0),
                    EngineID(0),
                    transport_types::enum_::VehicleKind::Ship,
                    TileIndex::new(0, 0),
                    list_id,
                ),
            );
        }

        sim.tick();
        // All vehicles must have been processed into Waiting(9)
        for id in [VehicleID(1), VehicleID(2), VehicleID(15), VehicleID(40)] {
            let v = sim.world.vehicles.get(&id).unwrap();
            assert_eq!(v.state, VehicleState::Waiting(9));
        }
    }

    #[test]
    fn test_end_to_end_ship_transport_slice() {
        let mut sim = Simulator::new(MapSize::new(10, 10));

        // 1. Create company with $20,000
        let comp_res = sim.process_command(Command::CreateCompany {
            name: "Harbor Express".to_string(),
            money: Money(20000),
            color: 0x0000FF,
        });
        let company_id = match comp_res {
            CommandResult::Success(CommandSuccess::CompanyCreated(id)) => id,
            _ => panic!("Expected company created"),
        };

        // 2. Build Dock A at (2, 2)
        let dock_a_res = sim.process_command(Command::BuildStation {
            company_id,
            name: "Port Alpha".to_string(),
            tiles: vec![TileIndex::new(2, 2)],
        });
        let dock_a_id = match dock_a_res {
            CommandResult::Success(CommandSuccess::StationBuilt(id)) => id,
            _ => panic!("Expected station built"),
        };

        // 3. Build Dock B at (6, 2)
        let dock_b_res = sim.process_command(Command::BuildStation {
            company_id,
            name: "Port Beta".to_string(),
            tiles: vec![TileIndex::new(6, 2)],
        });
        let dock_b_id = match dock_b_res {
            CommandResult::Success(CommandSuccess::StationBuilt(id)) => id,
            _ => panic!("Expected station built"),
        };

        // Station indexing: map and world must agree on station locations
        assert_eq!(
            sim.world.map.station_at(TileIndex::new(2, 2)),
            Some(dock_a_id)
        );
        assert_eq!(
            sim.world.map.station_at(TileIndex::new(6, 2)),
            Some(dock_b_id)
        );
        assert_eq!(
            sim.world
                .get_station_at_tile(&TileIndex::new(2, 2))
                .map(|s| s.id),
            Some(dock_a_id)
        );
        assert_eq!(
            sim.world
                .get_station_at_tile(&TileIndex::new(6, 2))
                .map(|s| s.id),
            Some(dock_b_id)
        );

        // Stock Dock A with 30 units of Grain (CargoType 1)
        let initial_cargo = 30u32;
        let station_a = sim.world.stations.get_mut(&dock_a_id).unwrap();
        if let Some(goods) = station_a
            .goods
            .iter_mut()
            .find(|g| g.cargo_type == CargoType(1))
        {
            goods.amount = CargoAmount(initial_cargo);
        } else {
            let mut g = GoodsEntry::new(CargoType(1));
            g.amount = CargoAmount(initial_cargo);
            station_a.goods.push(g);
        }

        // 4. Create OrderList: GoToStation(B), then GoToStation(A)
        let list_res = sim.process_command(Command::CreateOrderList {});
        let order_list_id = match list_res {
            CommandResult::Success(CommandSuccess::OrderListCreated(id)) => id,
            _ => panic!("Expected order list created"),
        };
        sim.process_command(Command::AddOrderToList {
            order_list_id,
            order: OrderType::GoToStation {
                station_id: dock_a_id,
                conditions: None,
            },
        });
        sim.process_command(Command::AddOrderToList {
            order_list_id,
            order: OrderType::GoToStation {
                station_id: dock_b_id,
                conditions: None,
            },
        });

        // 5. Purchase Ship at Dock A (2, 2)
        let balance_before_ship = sim.world.companies.get(&company_id).unwrap().money;
        let ship_res = sim.process_command(Command::PurchaseVehicle {
            company_id,
            engine_id: EngineID(1),
            kind: transport_types::enum_::VehicleKind::Ship,
            position: TileIndex::new(2, 2),
            order_list_id,
        });
        let vehicle_id = match ship_res {
            CommandResult::Success(CommandSuccess::VehiclePurchased(id)) => id,
            _ => panic!("Expected ship purchased"),
        };
        let balance_after_ship = sim.world.companies.get(&company_id).unwrap().money;
        assert_eq!(balance_before_ship.0 - balance_after_ship.0, 5000);

        // 6. First tick at Dock A: vehicle should transition to Loading and load the 30 units of grain
        sim.tick();
        let ship = sim.world.vehicles.get(&vehicle_id).unwrap();
        let ship_cargo: u32 = ship.cargo.iter().map(|(_, a)| a.0).sum();
        let dock_a_waiting = sim
            .world
            .stations
            .get(&dock_a_id)
            .unwrap()
            .goods
            .iter()
            .find(|g| g.cargo_type == CargoType(1))
            .unwrap()
            .amount
            .0;
        assert_eq!(ship_cargo, 30);
        assert_eq!(dock_a_waiting, 0);

        // Advance simulation until ship arrives at Dock B
        for _ in 0..10 {
            sim.tick();
            if sim.world.vehicles.get(&vehicle_id).unwrap().position == TileIndex::new(6, 2) {
                break;
            }
        }
        let ship_at_dest = sim.world.vehicles.get(&vehicle_id).unwrap();
        assert_eq!(ship_at_dest.position, TileIndex::new(6, 2));

        // Advance 1 more tick to process cargo delivery and unloading
        sim.tick();

        // 7. Verify delivery, company revenue, and cargo conservation
        let dock_b_delivered = sim
            .world
            .stations
            .get(&dock_b_id)
            .unwrap()
            .goods
            .iter()
            .find(|g| g.cargo_type == CargoType(1))
            .map(|g| g.delivered_since_last_visit.0)
            .unwrap_or(0);
        assert_eq!(
            dock_b_delivered, 30,
            "All 30 units must be delivered at Dock B"
        );

        // Company must have received 30 * 10 = $300 revenue
        let final_balance = sim.world.companies.get(&company_id).unwrap().money;
        assert_eq!(final_balance.0, balance_after_ship.0 + 300);

        // Cargo Conservation Invariant: total cargo units never created or destroyed
        let waiting_total: u32 = sim
            .world
            .stations
            .values()
            .flat_map(|s| &s.goods)
            .map(|g| g.amount.0)
            .sum();
        let on_vehicle_total: u32 = sim
            .world
            .vehicles
            .values()
            .flat_map(|v| &v.cargo)
            .map(|(_, a)| a.0)
            .sum();
        let delivered_total: u32 = sim
            .world
            .stations
            .values()
            .flat_map(|s| &s.goods)
            .map(|g| g.delivered_since_last_visit.0)
            .sum();
        assert_eq!(
            waiting_total + on_vehicle_total + delivered_total,
            initial_cargo
        );
    }

    #[test]
    fn test_station_tile_overlap_prevention() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let company_res = sim.process_command(Command::CreateCompany {
            name: "Port Authority".to_string(),
            money: Money(10000),
            color: 0xFF0000,
        });
        let company_id = match company_res {
            CommandResult::Success(CommandSuccess::CompanyCreated(id)) => id,
            _ => panic!("Expected company created"),
        };

        // Build first station at (3, 3)
        let res1 = sim.process_command(Command::BuildStation {
            company_id,
            name: "First Dock".to_string(),
            tiles: vec![TileIndex::new(3, 3)],
        });
        assert!(res1.is_success());

        // Attempt to build second station overlapping (3, 3)
        let res2 = sim.process_command(Command::BuildStation {
            company_id,
            name: "Overlapping Dock".to_string(),
            tiles: vec![TileIndex::new(3, 3)],
        });
        assert_eq!(
            res2,
            CommandResult::Failure(CommandFailure::TileOccupied(TileIndex::new(3, 3)))
        );
        assert_eq!(sim.world.stations.len(), 1);
    }

    #[test]
    fn test_insufficient_funds_rejection() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let company_res = sim.process_command(Command::CreateCompany {
            name: "Broke Shipping".to_string(),
            money: Money(500), // only $500 available
            color: 0x00FF00,
        });
        let company_id = match company_res {
            CommandResult::Success(CommandSuccess::CompanyCreated(id)) => id,
            _ => panic!("Expected company created"),
        };

        // Station costs $1000 per tile -> must fail with InsufficientFunds
        let res = sim.process_command(Command::BuildStation {
            company_id,
            name: "Expensive Dock".to_string(),
            tiles: vec![TileIndex::new(1, 1)],
        });
        assert_eq!(
            res,
            CommandResult::Failure(CommandFailure::InsufficientFunds {
                required: Money(1000),
                available: Money(500),
            })
        );
        assert_eq!(sim.world.stations.len(), 0);
        assert_eq!(
            sim.world.companies.get(&company_id).unwrap().money,
            Money(500)
        );
    }
}
