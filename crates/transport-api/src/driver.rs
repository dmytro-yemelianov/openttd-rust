use std::sync::Arc;
use transport_sim::kernel::{DriverError, KernelContext, Phase, SubsystemDriver};
use transport_sim::Command;
use transport_types::{CapabilityToken, CompanyID, ServiceId, ServicePriority};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};

use crate::ingress::CommandQueue;
use crate::ring::EventRingBuffer;
use crate::Event;

/// API subsystem driver integrating client command ingress and event egress
/// into the microkernel tick cycle.
pub struct ApiSubsystemDriver {
    command_queue: Arc<CommandQueue>,
    event_ring: Arc<EventRingBuffer<Event>>,
}

impl ApiSubsystemDriver {
    /// Create a new API driver with shared command queue and event ring buffer.
    pub fn new(
        command_queue: Arc<CommandQueue>,
        event_ring: Arc<EventRingBuffer<Event>>,
    ) -> Self {
        Self {
            command_queue,
            event_ring,
        }
    }

    /// Access the shared command queue for external submissions.
    pub fn command_queue(&self) -> &Arc<CommandQueue> {
        &self.command_queue
    }

    /// Access the shared event ring buffer for external subscribers.
    pub fn event_ring(&self) -> &Arc<EventRingBuffer<Event>> {
        &self.event_ring
    }

    /// Execute command ingress during `Phase::Ingress`.
    fn handle_ingress(&self, ctx: &mut KernelContext) -> Result<(), DriverError> {
        let commands = self.command_queue.drain_for_tick();

        for cmd in commands {
            // 1. Verify capability token
            if !cmd.capability_token.can_mutate_company(cmd.company_id)
                && !cmd.capability_token.is_system()
            {
                // Unauthorized command: rejected deterministically without mutating world
                continue;
            }

            // 2. Execute authorized command
            self.execute_command(&cmd.command, ctx);
        }

        Ok(())
    }

    /// Execute command mutations against the world state.
    fn execute_command(&self, command: &Command, ctx: &mut KernelContext) {
        match command {
            Command::CreateCompany { name, money, color } => {
                let id = CompanyID((ctx.world.companies.len() + 1) as u32);
                let company = Company::new(id, name.clone(), *money, *color);
                ctx.world.companies.insert(id, company);
            }
            Command::BuildStation {
                company_id,
                name,
                tiles,
            } => {
                if !ctx.world.companies.contains_key(company_id) || tiles.is_empty() {
                    return;
                }
                let station_id = transport_types::StationID((ctx.world.stations.len() + 1) as u32);
                let mut st = Station::new(station_id, name.clone(), tiles.clone());
                st.company_id = Some(*company_id);

                for tile in tiles {
                    if ctx.world.map.size().is_valid_index(*tile) {
                        let _ = ctx.world.map.set_station_at(*tile, station_id, Some(*company_id));
                        ctx.world.tile_to_station.insert(*tile, station_id);
                    }
                }
                ctx.world.stations.insert(station_id, st);
            }
            Command::PurchaseVehicle {
                company_id,
                engine_id,
                kind,
                position,
                order_list_id,
            } => {
                if !ctx.world.companies.contains_key(company_id) {
                    return;
                }
                let vehicle_id = transport_types::VehicleID((ctx.world.vehicles.len() + 1) as u32);
                let veh = Vehicle::new(
                    vehicle_id,
                    *company_id,
                    *engine_id,
                    *kind,
                    *position,
                    *order_list_id,
                );
                ctx.world.vehicles.insert(vehicle_id, veh);
            }
            Command::CreateOrderList {} => {
                let id = transport_types::OrderListID((ctx.world.order_lists.len() + 1) as u32);
                ctx.world.order_lists.insert(id, Vec::new());
            }
            Command::AddOrderToList {
                order_list_id,
                order,
            } => {
                if let Some(list) = ctx.world.order_lists.get_mut(order_list_id) {
                    list.push(order.clone());
                }
            }
            Command::LoadCargo {
                vehicle_id,
                station_id,
                cargo_type,
                amount,
            } => {
                ctx.stage_intent(
                    CapabilityToken::System,
                    transport_sim::KernelIntent::TransferCargo {
                        vehicle_id: *vehicle_id,
                        station_id: *station_id,
                        cargo_type: *cargo_type,
                        amount: *amount,
                        is_load: true,
                    },
                );
            }
            Command::UnloadCargo {
                vehicle_id,
                station_id,
                cargo_type,
                amount,
            } => {
                ctx.stage_intent(
                    CapabilityToken::System,
                    transport_sim::KernelIntent::TransferCargo {
                        vehicle_id: *vehicle_id,
                        station_id: *station_id,
                        cargo_type: *cargo_type,
                        amount: *amount,
                        is_load: false,
                    },
                );
            }
        }
    }

    /// Publish simulation events into the ring buffer during `Phase::Egress`.
    fn handle_egress(&self, ctx: &mut KernelContext) -> Result<(), DriverError> {
        for ke in ctx.events.iter() {
            let event = match ke {
                transport_sim::KernelEvent::VehicleMoved {
                    vehicle_id,
                    old_tile,
                    new_tile,
                    tick,
                } => Event::VehicleMoved {
                    vehicle_id: *vehicle_id,
                    old_tile: *old_tile,
                    new_tile: *new_tile,
                    tick: *tick,
                },
                transport_sim::KernelEvent::CargoTransferred {
                    vehicle_id,
                    station_id,
                    cargo_type,
                    amount,
                    is_load: true,
                    tick,
                } => Event::CargoLoaded {
                    vehicle_id: *vehicle_id,
                    station_id: *station_id,
                    cargo_type: *cargo_type,
                    amount: *amount,
                    tick: *tick,
                },
                transport_sim::KernelEvent::CargoTransferred {
                    vehicle_id,
                    station_id,
                    cargo_type,
                    amount,
                    is_load: false,
                    tick,
                } => Event::CargoUnloaded {
                    vehicle_id: *vehicle_id,
                    station_id: *station_id,
                    cargo_type: *cargo_type,
                    amount: *amount,
                    tick: *tick,
                },
            };

            self.event_ring.push(event);
        }

        Ok(())
    }
}

impl SubsystemDriver for ApiSubsystemDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Api
    }

    fn priority(&self) -> ServicePriority {
        // High priority: executes first in Ingress, and first in Egress
        ServicePriority::HIGH
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        match phase {
            Phase::Ingress => self.handle_ingress(ctx),
            Phase::Egress => self.handle_egress(ctx),
            _ => Ok(()),
        }
    }
}
