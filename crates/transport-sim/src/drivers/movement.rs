use crate::kernel::context::KernelContext;
use crate::kernel::driver::{DriverError, SubsystemDriver};
use crate::kernel::intent::KernelIntent;
use crate::kernel::phase::Phase;
use crate::step_towards;
use transport_types::enum_::{OrderType, VehicleState};
use transport_types::{CapabilityToken, OrderIndex, ServiceId, ServicePriority, VehicleID};

/// Subsystem driver responsible for vehicle movement, order traversal, and single-tile stepping.
pub struct MovementDriver;

impl MovementDriver {
    pub fn new() -> Self {
        Self
    }

    fn advance_vehicle(vehicle_id: VehicleID, ctx: &mut KernelContext) {
        let (orders_id, vehicle_pos, vehicle_state, current_order_opt, company_id, planned_waypoint) = {
            let vehicle = match ctx.world.vehicles.get(&vehicle_id) {
                Some(v) => v,
                None => return,
            };
            (
                vehicle.orders,
                vehicle.position,
                vehicle.state,
                vehicle.current_order,
                vehicle.company_id,
                vehicle.next_waypoint,
            )
        };

        let orders = match ctx.world.order_lists.get(&orders_id) {
            Some(list) if !list.is_empty() => list,
            _ => {
                if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.current_order = None;
                    v.state = VehicleState::Idle;
                }
                return;
            }
        };
        let orders_len = orders.len();

        let current_idx = match current_order_opt {
            Some(idx) if (idx.0 as usize) < orders_len => idx.0 as usize,
            _ => {
                if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.current_order = Some(OrderIndex(0));
                    v.state = VehicleState::Idle;
                }
                0
            }
        };

        let order = match orders.get(current_idx) {
            Some(o) => o.clone(),
            None => return,
        };

        match order {
            OrderType::WaitTime(ticks) => {
                let (next_order, next_state) = match vehicle_state {
                    VehicleState::Waiting(rem) => {
                        if rem <= 1 {
                            let next_idx = (current_idx + 1) % orders_len;
                            (Some(OrderIndex(next_idx as u16)), VehicleState::Idle)
                        } else {
                            (current_order_opt, VehicleState::Waiting(rem - 1))
                        }
                    }
                    _ => {
                        if ticks <= 1 {
                            let next_idx = (current_idx + 1) % orders_len;
                            (Some(OrderIndex(next_idx as u16)), VehicleState::Idle)
                        } else {
                            (current_order_opt, VehicleState::Waiting(ticks - 1))
                        }
                    }
                };
                if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.current_order = next_order;
                    v.state = next_state;
                }
            }
            OrderType::GoToStation { station_id, .. } => {
                let station_opt = ctx.world.stations.get(&station_id);
                let (is_at_station, target_tile) = match station_opt {
                    Some(s) => (s.tiles.contains(&vehicle_pos), s.tiles.first().copied()),
                    None => {
                        if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                            v.state = VehicleState::Idle;
                        }
                        return;
                    }
                };

                if is_at_station {
                    let next_idx = (current_idx + 1) % orders_len;
                    if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                        v.current_order = Some(OrderIndex(next_idx as u16));
                        v.state = VehicleState::Loading;
                    }
                } else if let Some(target) = target_tile {
                    let next_pos = if let Some(waypoint) = planned_waypoint {
                        waypoint
                    } else {
                        step_towards(vehicle_pos, target, &ctx.world.map)
                    };
                    let arrived = ctx
                        .world
                        .stations
                        .get(&station_id)
                        .is_some_and(|s| s.tiles.contains(&next_pos));
                    let (next_order, next_state) = if arrived {
                        let next_idx = (current_idx + 1) % orders_len;
                        (Some(OrderIndex(next_idx as u16)), VehicleState::Loading)
                    } else {
                        (current_order_opt, VehicleState::Traveling)
                    };

                    if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                        v.position = next_pos;
                        v.current_order = next_order;
                        v.state = next_state;
                        v.next_waypoint = None;
                    }

                    // Stage move intent for capability verification & audit
                    ctx.stage_intent(
                        CapabilityToken::Company(company_id),
                        KernelIntent::MoveVehicle {
                            vehicle_id,
                            to_tile: next_pos,
                        },
                    );
                    ctx.stage_intent(
                        CapabilityToken::Company(company_id),
                        KernelIntent::EmitEvent(crate::kernel::KernelEvent::VehicleMoved {
                            vehicle_id,
                            old_tile: vehicle_pos,
                            new_tile: next_pos,
                            tick: ctx.tick,
                        }),
                    );
                }
            }
            _ => {}
        }
    }
}

impl Default for MovementDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for MovementDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Movement
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::HIGH
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Drivers {
            return Ok(());
        }

        ctx.vehicle_order_buffer.clear();
        ctx.vehicle_order_buffer
            .extend(ctx.world.vehicles.keys().copied());
        ctx.vehicle_order_buffer.sort_unstable();

        for i in 0..ctx.vehicle_order_buffer.len() {
            let vehicle_id = ctx.vehicle_order_buffer[i];
            Self::advance_vehicle(vehicle_id, ctx);
        }

        Ok(())
    }
}
