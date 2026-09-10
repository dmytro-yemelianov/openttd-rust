use crate::kernel::context::KernelContext;
use crate::kernel::driver::{DriverError, SubsystemDriver};
use crate::kernel::intent::KernelIntent;
use crate::kernel::phase::Phase;
use transport_types::enum_::VehicleState;
use transport_types::{CapabilityToken, CargoAmount, Money, ServiceId, ServicePriority};
use transport_world::definitions::GoodsEntry;

/// Subsystem driver responsible for station cargo loading, unloading, and capacity limits.
pub struct LogisticsDriver;

impl LogisticsDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogisticsDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for LogisticsDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Logistics
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::NORMAL
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Drivers {
            return Ok(());
        }

        for i in 0..ctx.vehicle_order_buffer.len() {
            let vehicle_id = ctx.vehicle_order_buffer[i];
            let (is_loading, pos, company_id) = match ctx.world.vehicles.get(&vehicle_id) {
                Some(v) => (v.state == VehicleState::Loading, v.position, v.company_id),
                None => continue,
            };
            if !is_loading {
                continue;
            }

            let station_id = match ctx.world.get_station_at_tile(&pos) {
                Some(s) => s.id,
                None => continue,
            };

            // 1. Unload cargo
            let mut unloaded = Vec::new();
            if let Some(vehicle) = ctx.world.vehicles.get_mut(&vehicle_id) {
                for (cargo_type, amount) in vehicle.cargo.drain(..) {
                    if amount.0 > 0 {
                        unloaded.push((cargo_type, amount));
                    }
                }
            }

            for (cargo_type, amount) in unloaded {
                if let Some(station) = ctx.world.stations.get_mut(&station_id) {
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
                // Stage revenue intent via Capability token
                let revenue = Money(amount.0 as i64 * 10);
                ctx.stage_intent(
                    CapabilityToken::Company(company_id),
                    KernelIntent::CreditRevenue {
                        company_id,
                        amount: revenue,
                    },
                );
            }

            // 2. Load cargo from station
            let capacity = 50u32;
            let current_load: u32 = ctx
                .world
                .vehicles
                .get(&vehicle_id)
                .map(|v| v.cargo.iter().map(|(_, a)| a.0).sum())
                .unwrap_or(0);
            let available_cap = capacity.saturating_sub(current_load);
            if available_cap > 0 {
                if let Some(station) = ctx.world.stations.get_mut(&station_id) {
                    for goods in &mut station.goods {
                        if goods.amount.0 > 0 {
                            let transfer = std::cmp::min(goods.amount.0, available_cap);
                            goods.amount.0 -= transfer;
                            if let Some(vehicle) = ctx.world.vehicles.get_mut(&vehicle_id) {
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
}
