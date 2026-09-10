use std::collections::BTreeMap;
use transport_sim::kernel::context::KernelContext;
use transport_sim::kernel::driver::{DriverError, SubsystemDriver};
use transport_sim::kernel::intent::KernelIntent;
use transport_sim::kernel::phase::Phase;
use transport_types::enum_::VehicleState;
use transport_types::{CapabilityToken, Money, PersonID, ServiceId, ServicePriority, StationID, VehicleID};
use crate::manifest::VehicleManifest;
use crate::peep::{Peep, PeepState, StaffRole};
use crate::turnstile::StationTurnstile;

/// Subsystem driver managing OpenRCT2-style Peeps, station turnstiles, and passenger manifests.
pub struct PeopleDriver {
    pub peeps: BTreeMap<PersonID, Peep>,
    pub turnstiles: BTreeMap<StationID, StationTurnstile>,
    pub manifests: BTreeMap<VehicleID, VehicleManifest>,
}

impl PeopleDriver {
    pub fn new() -> Self {
        Self {
            peeps: BTreeMap::new(),
            turnstiles: BTreeMap::new(),
            manifests: BTreeMap::new(),
        }
    }

    /// Register a new peep into the simulation.
    pub fn add_peep(&mut self, peep: Peep) {
        self.peeps.insert(peep.id, peep);
    }

    /// Register or configure a station turnstile with a ticket fare.
    pub fn configure_turnstile(&mut self, station_id: StationID, ticket_fare: Money) {
        self.turnstiles
            .entry(station_id)
            .or_insert_with(|| StationTurnstile::new(station_id, ticket_fare))
            .ticket_fare = ticket_fare;
    }

    /// Register or configure a vehicle manifest with passenger capacity.
    pub fn configure_manifest(&mut self, vehicle_id: VehicleID, capacity: usize) {
        self.manifests
            .entry(vehicle_id)
            .or_insert_with(|| VehicleManifest::new(vehicle_id, capacity))
            .passenger_capacity = capacity;
    }

    /// Assign a crew member to a vehicle.
    pub fn assign_crew(&mut self, vehicle_id: VehicleID, person_id: PersonID, role: StaffRole) {
        if let Some(peep) = self.peeps.get_mut(&person_id) {
            peep.role = role;
            peep.state = PeepState::AboardVehicle { vehicle_id };
        }
        self.manifests
            .entry(vehicle_id)
            .or_insert_with(|| VehicleManifest::new(vehicle_id, 20))
            .add_crew(person_id, role);
    }
}

impl Default for PeopleDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for PeopleDriver {
    fn id(&self) -> ServiceId {
        ServiceId::People
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::NORMAL
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Drivers {
            return Ok(());
        }

        // 1. Process pedestrians walking to their target station turnstile
        for peep in self.peeps.values_mut() {
            if let PeepState::WalkingToStation { target_station } = peep.state {
                let station_tiles = ctx.world.stations.get(&target_station).map(|s| &s.tiles);
                let arrived = station_tiles.is_some_and(|tiles| tiles.contains(&peep.current_tile));
                if arrived {
                    peep.state = PeepState::InStationQueue { station_id: target_station };
                }
            }
        }

        // Enqueue newly arrived peeps into the turnstile queues
        for peep in self.peeps.values() {
            if let PeepState::InStationQueue { station_id } = peep.state {
                if let Some(turnstile) = self.turnstiles.get_mut(&station_id) {
                    if !turnstile.queue.contains(&peep.id) {
                        turnstile.enqueue(peep.id);
                    }
                }
            }
        }

        // 2. Identify docked vehicles and process Alighting & Turnstile Boarding
        for i in 0..ctx.vehicle_order_buffer.len() {
            let vehicle_id = ctx.vehicle_order_buffer[i];
            let (is_loading, pos, company_id) = match ctx.world.vehicles.get(&vehicle_id) {
                Some(v) => (v.state == VehicleState::Loading, v.position, v.company_id),
                None => continue,
            };

            let station_id = match ctx.world.get_station_at_tile(&pos) {
                Some(s) => s.id,
                None => continue,
            };

            // Ensure manifest exists
            let manifest = self
                .manifests
                .entry(vehicle_id)
                .or_insert_with(|| VehicleManifest::new(vehicle_id, 30));

            // Step A: Alight passengers whose destination is this station
            let mut alighting_passengers = Vec::new();
            for &p_id in &manifest.passengers {
                if let Some(peep) = self.peeps.get(&p_id) {
                    if peep.destination_station == station_id {
                        alighting_passengers.push(p_id);
                    }
                }
            }

            for p_id in alighting_passengers {
                manifest.remove_passenger(p_id);
                if let Some(peep) = self.peeps.get_mut(&p_id) {
                    peep.current_tile = pos;
                    peep.state = PeepState::Completed;
                }
            }

            // Step B: If vehicle is loading, board waiting passengers through turnstile
            if is_loading {
                let fare = self.turnstiles.get(&station_id).map(|t| t.ticket_fare).unwrap_or(Money(0));
                let mut admitted = Vec::new();

                if let Some(turnstile) = self.turnstiles.get_mut(&station_id) {
                    while manifest.available_seats() > 0 && turnstile.queue_len() > 0 {
                        if let Some(candidate_id) = turnstile.pop_front() {
                            admitted.push(candidate_id);
                            manifest.add_passenger(candidate_id);
                        }
                    }
                }

                // Stage fare revenue and update admitted peeps
                for p_id in admitted {
                    if let Some(peep) = self.peeps.get_mut(&p_id) {
                        peep.wallet = peep.wallet.saturating_sub(fare);
                        peep.fare_paid = peep.fare_paid.saturating_add(fare);
                        peep.current_tile = pos;
                        peep.state = PeepState::AboardVehicle { vehicle_id };
                    }
                    if fare.0 > 0 {
                        ctx.stage_intent(
                            CapabilityToken::Company(company_id),
                            KernelIntent::CreditRevenue {
                                company_id,
                                amount: fare,
                            },
                        );
                    }
                }
            }
        }

        // Update positions of passengers aboard vehicles in transit
        for (vehicle_id, manifest) in &self.manifests {
            if let Some(vehicle) = ctx.world.vehicles.get(vehicle_id) {
                for &p_id in &manifest.passengers {
                    if let Some(peep) = self.peeps.get_mut(&p_id) {
                        peep.current_tile = vehicle.position;
                    }
                }
                for (c_id, _) in &manifest.crew {
                    if let Some(peep) = self.peeps.get_mut(c_id) {
                        peep.current_tile = vehicle.position;
                    }
                }
            }
        }

        Ok(())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
