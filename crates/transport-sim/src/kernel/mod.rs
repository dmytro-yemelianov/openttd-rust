pub mod context;
pub mod driver;
pub mod event;
pub mod intent;
pub mod phase;

pub use context::KernelContext;
pub use driver::{DriverError, SubsystemDriver};
pub use event::KernelEvent;
pub use intent::KernelIntent;
pub use phase::Phase;

use crate::World;
use transport_types::{CapabilityToken, VehicleID};
use transport_world::map::MapSize;

/// Deterministic capability-based microkernel coordinator.
pub struct Microkernel {
    pub world: World,
    pub drivers: Vec<Box<dyn SubsystemDriver>>,
    pub vehicle_order_buffer: Vec<VehicleID>,
    pub intent_buffer: Vec<(CapabilityToken, KernelIntent)>,
    pub event_buffer: Vec<KernelEvent>,
}

impl Microkernel {
    /// Create a new microkernel with an empty world of specified dimensions.
    pub fn new(size: MapSize) -> Self {
        Self {
            world: World::new(size),
            drivers: Vec::new(),
            vehicle_order_buffer: Vec::new(),
            intent_buffer: Vec::new(),
            event_buffer: Vec::new(),
        }
    }

    /// Register a subsystem driver. Drivers will be executed in deterministic priority order.
    pub fn register_driver<D: SubsystemDriver + 'static>(&mut self, driver: D) {
        self.drivers.push(Box::new(driver));
        // Sort drivers deterministically by priority: lower numerical value first
        self.drivers.sort_by_key(|d| d.priority());
    }

    /// Execute a single deterministic tick through the 5 microkernel phases.
    pub fn tick(&mut self) -> Result<(), DriverError> {
        self.intent_buffer.clear();
        self.event_buffer.clear();

        // 1. Phase::Ingress
        self.dispatch_phase(Phase::Ingress)?;

        // 2. Phase::Solvers
        self.dispatch_phase(Phase::Solvers)?;

        // 3. Phase::Drivers
        self.dispatch_phase(Phase::Drivers)?;

        // 4. Phase::Commit (atomic transactional state commit)
        self.commit_intents()?;

        // 5. Phase::Egress
        self.dispatch_phase(Phase::Egress)?;

        // Advance monotonic tick
        self.world.advance_tick();
        Ok(())
    }

    /// Dispatch a specific execution phase to all registered subsystem drivers in priority order.
    fn dispatch_phase(&mut self, phase: Phase) -> Result<(), DriverError> {
        let current_tick = self.world.tick;
        for i in 0..self.drivers.len() {
            let mut ctx = KernelContext {
                world: &mut self.world,
                tick: current_tick,
                vehicle_order_buffer: &mut self.vehicle_order_buffer,
                intents: &mut self.intent_buffer,
                events: &mut self.event_buffer,
            };
            self.drivers[i].execute_phase(phase, &mut ctx)?;
        }
        Ok(())
    }

    /// Transactionally commit staged intents after validating capability permissions.
    fn commit_intents(&mut self) -> Result<(), DriverError> {
        for (token, intent) in self.intent_buffer.drain(..) {
            match intent {
                KernelIntent::CreditRevenue { company_id, amount } => {
                    if !token.can_mutate_company(company_id) {
                        return Err(DriverError::Unauthorized(token));
                    }
                    if let Some(company) = self.world.companies.get_mut(&company_id) {
                        company.money = company.money.saturating_add(amount);
                    }
                }
                KernelIntent::DeductCost { company_id, amount } => {
                    if !token.can_mutate_company(company_id) {
                        return Err(DriverError::Unauthorized(token));
                    }
                    if let Some(company) = self.world.companies.get_mut(&company_id) {
                        company.money = company.money.saturating_sub(amount);
                    }
                }
                KernelIntent::MoveVehicle {
                    vehicle_id,
                    to_tile,
                } => {
                    if let Some(vehicle) = self.world.vehicles.get_mut(&vehicle_id) {
                        if !token.can_mutate_company(vehicle.company_id) {
                            return Err(DriverError::Unauthorized(token));
                        }
                        if self.world.map.size().is_valid_index(to_tile) {
                            vehicle.position = to_tile;
                        }
                    }
                }
                KernelIntent::TransferCargo {
                    vehicle_id,
                    station_id,
                    cargo_type,
                    amount,
                    is_load,
                } => {
                    let (veh_company, can_proceed) =
                        if let Some(vehicle) = self.world.vehicles.get(&vehicle_id) {
                            (
                                vehicle.company_id,
                                token.can_mutate_company(vehicle.company_id),
                            )
                        } else {
                            continue;
                        };
                    if !can_proceed {
                        return Err(DriverError::Unauthorized(token));
                    }

                    if is_load {
                        // Station -> Vehicle
                        let to_load =
                            if let Some(station) = self.world.stations.get_mut(&station_id) {
                                if let Some(goods) = station
                                    .goods
                                    .iter_mut()
                                    .find(|g| g.cargo_type == cargo_type)
                                {
                                    let transfer = std::cmp::min(goods.amount.0, amount.0);
                                    goods.amount.0 -= transfer;
                                    transport_types::CargoAmount(transfer)
                                } else {
                                    transport_types::CargoAmount(0)
                                }
                            } else {
                                transport_types::CargoAmount(0)
                            };

                        if to_load.0 > 0 {
                            if let Some(vehicle) = self.world.vehicles.get_mut(&vehicle_id) {
                                if let Some(e) =
                                    vehicle.cargo.iter_mut().find(|(c, _)| *c == cargo_type)
                                {
                                    e.1 .0 += to_load.0;
                                } else {
                                    vehicle.cargo.push((cargo_type, to_load));
                                }
                            }
                        }
                    } else {
                        // Vehicle -> Station
                        let to_unload =
                            if let Some(vehicle) = self.world.vehicles.get_mut(&vehicle_id) {
                                if let Some(e) =
                                    vehicle.cargo.iter_mut().find(|(c, _)| *c == cargo_type)
                                {
                                    let transfer = std::cmp::min(e.1 .0, amount.0);
                                    e.1 .0 -= transfer;
                                    transport_types::CargoAmount(transfer)
                                } else {
                                    transport_types::CargoAmount(0)
                                }
                            } else {
                                transport_types::CargoAmount(0)
                            };

                        if to_unload.0 > 0 {
                            if let Some(station) = self.world.stations.get_mut(&station_id) {
                                if let Some(g) = station
                                    .goods
                                    .iter_mut()
                                    .find(|g| g.cargo_type == cargo_type)
                                {
                                    g.delivered_since_last_visit.0 += to_unload.0;
                                } else {
                                    let mut g =
                                        transport_world::definitions::GoodsEntry::new(cargo_type);
                                    g.delivered_since_last_visit = to_unload;
                                    station.goods.push(g);
                                }
                            }
                            if let Some(company) = self.world.companies.get_mut(&veh_company) {
                                company.money = company.money.saturating_add(
                                    transport_types::Money(to_unload.0 as i64 * 10),
                                );
                            }
                        }
                    }
                }
                KernelIntent::EmitEvent(event) => {
                    self.event_buffer.push(event);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
