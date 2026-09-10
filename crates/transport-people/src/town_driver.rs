use std::collections::BTreeMap;
use transport_sim::kernel::{DriverError, KernelContext, KernelIntent, Phase, SubsystemDriver};
use transport_types::{
    BuildingID, CapabilityToken, Money, PersonID, ServiceId, ServicePriority, StationID, TileIndex,
    TownID,
};

use crate::commute::CommutePlan;
use crate::peep::{Peep, PeepState};
use crate::turnstile::StationTurnstile;

/// Hard upper bound on active materialized commuters to guarantee bounded tick latency.
pub const DEFAULT_MAX_ACTIVE_PEEPS: usize = 8192;

/// Subsystem driver simulating the macro-urban economy, commute generation,
/// turnstile fare admissions, and fulfillment-driven town expansion.
pub struct TownEconomyDriver {
    pub active_commuters: BTreeMap<PersonID, (Peep, CommutePlan)>,
    pub turnstiles: BTreeMap<StationID, StationTurnstile>,
    pub max_active_peeps: usize,
    pub next_person_id: u64,
    pub next_building_id: u32,
    pub default_fare: Money,
}

impl Default for TownEconomyDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl TownEconomyDriver {
    pub fn new() -> Self {
        Self {
            active_commuters: BTreeMap::new(),
            turnstiles: BTreeMap::new(),
            max_active_peeps: DEFAULT_MAX_ACTIVE_PEEPS,
            next_person_id: 1,
            next_building_id: 100,
            default_fare: Money(5),
        }
    }

    /// Register a station turnstile charging a specific ticket fare.
    pub fn register_turnstile(&mut self, station_id: StationID, fare: Money) {
        self.turnstiles
            .insert(station_id, StationTurnstile::new(station_id, fare));
    }

    /// Find an existing station within walking radius (<= 3 Manhattan distance) of a tile.
    fn find_nearby_station(&self, tile: TileIndex, ctx: &KernelContext) -> Option<StationID> {
        for (sid, st) in &ctx.world.stations {
            for s_tile in &st.tiles {
                let dx = (s_tile.x as i32 - tile.x as i32).abs();
                let dy = (s_tile.y as i32 - tile.y as i32).abs();
                if (dx + dy) <= 3 {
                    return Some(*sid);
                }
            }
        }
        None
    }

    /// Phase::Solvers — Generate commute demands and materialize active peeps.
    fn handle_solvers(&mut self, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if self.active_commuters.len() >= self.max_active_peeps {
            return Ok(());
        }

        // Collect residential and workplace buildings deterministically
        let mut residences = Vec::new();
        let mut workplaces = Vec::new();

        for b in ctx.world.buildings.values() {
            if b.is_residence() {
                residences.push(b.clone());
            } else if b.is_workplace() {
                workplaces.push(b.clone());
            }
        }

        if residences.is_empty() || workplaces.is_empty() {
            return Ok(());
        }

        // Generate commute trips for residences with nearby stations
        for res in &residences {
            if self.active_commuters.len() >= self.max_active_peeps {
                break;
            }

            let origin_station = match self.find_nearby_station(res.tile, ctx) {
                Some(s) => s,
                None => continue,
            };

            // Pair with a workplace that has a nearby destination station
            for work in &workplaces {
                if work.town_id == res.town_id && workplaces.len() > 1 {
                    // Prefer inter-town or distant workplaces
                    continue;
                }

                let dest_station = match self.find_nearby_station(work.tile, ctx) {
                    Some(s) => s,
                    None => continue,
                };

                if origin_station == dest_station {
                    // Zero transit needed if stations are the same
                    continue;
                }

                let person_id = PersonID(self.next_person_id);
                self.next_person_id += 1;

                let plan = CommutePlan::new(
                    res.id,
                    work.id,
                    res.tile,
                    work.tile,
                    Money(20), // Max willing to pay
                    ctx.tick,
                    400, // Max patience 400 ticks
                );

                let mut peep = Peep::new(
                    person_id,
                    res.tile,
                    dest_station,
                    Money(100),
                );
                peep.state = PeepState::InStationQueue { station_id: origin_station };

                // Enqueue into station turnstile
                if let Some(turnstile) = self.turnstiles.get_mut(&origin_station) {
                    turnstile.enqueue(person_id);
                }

                self.active_commuters.insert(person_id, (peep, plan));
                break;
            }
        }

        Ok(())
    }

    /// Phase::Drivers — Process turnstile admissions, fare collection, and destination arrivals.
    fn handle_drivers(&mut self, ctx: &mut KernelContext) -> Result<(), DriverError> {
        // 1. Process turnstile ticket admissions and fare deduction
        for (sid, turnstile) in &mut self.turnstiles {
            if let Some(person_id) = turnstile.pop_front() {
                if let Some((peep, plan)) = self.active_commuters.get_mut(&person_id) {
                    let fare = turnstile.ticket_fare;
                    if peep.wallet.0 >= fare.0 {
                        peep.wallet = peep.wallet.saturating_sub(fare);
                        peep.fare_paid = peep.fare_paid.saturating_add(fare);
                        plan.total_fare_paid = plan.total_fare_paid.saturating_add(fare);

                        // Credit revenue to the company operating the station
                        if let Some(st) = ctx.world.stations.get(sid) {
                            if let Some(cid) = st.company_id {
                                ctx.stage_intent(
                                    CapabilityToken::Company(cid),
                                    KernelIntent::CreditRevenue {
                                        company_id: cid,
                                        amount: fare,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // 2. Evaluate arrival at destination or trip abandonment
        let mut completed_or_abandoned = Vec::new();

        for (&person_id, (peep, plan)) in &self.active_commuters {
            let is_arrived = plan.is_at_destination(peep.current_tile);
            let is_expired = ctx.tick.0.saturating_sub(plan.start_tick.0) > plan.max_patience;

            if is_arrived {
                let score = plan.calculate_satisfaction(ctx.tick);
                completed_or_abandoned.push((person_id, true, score, plan.origin_building));
            } else if is_expired {
                completed_or_abandoned.push((person_id, false, 10, plan.origin_building));
            }
        }

        for (person_id, fulfilled, satisfaction, orig_bldg) in completed_or_abandoned {
            self.active_commuters.remove(&person_id);

            // Find origin town and record outcome
            if let Some(bldg) = ctx.world.buildings.get(&orig_bldg) {
                if let Some(town) = ctx.world.towns.get_mut(&bldg.town_id) {
                    town.record_commute(fulfilled, satisfaction);
                }
            }
        }

        Ok(())
    }

    /// Phase::Egress — Evaluate fulfillment-driven town growth pulses.
    fn handle_egress(&mut self, ctx: &mut KernelContext) -> Result<(), DriverError> {
        let mut town_ids: Vec<TownID> = ctx.world.towns.keys().copied().collect();
        town_ids.sort_unstable(); // Deterministic ordering

        let mut new_buildings = Vec::new();

        for tid in town_ids {
            if let Some(town) = ctx.world.towns.get_mut(&tid) {
                if let Some(bldg) = town.evaluate_growth(
                    &mut ctx.world.map,
                    BuildingID(self.next_building_id),
                    &ctx.world.buildings,
                ) {
                    self.next_building_id += 1;
                    new_buildings.push(bldg);
                }
            }
        }

        for bldg in new_buildings {
            ctx.world.buildings.insert(bldg.id, bldg);
        }

        Ok(())
    }
}

impl SubsystemDriver for TownEconomyDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Economy
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::NORMAL
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        match phase {
            Phase::Solvers => self.handle_solvers(ctx),
            Phase::Drivers => self.handle_drivers(ctx),
            Phase::Egress => self.handle_egress(ctx),
            _ => Ok(()),
        }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
