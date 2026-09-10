use serde::{Deserialize, Serialize};
use transport_types::{PersonID, VehicleID};
use crate::peep::StaffRole;

/// Vehicle passenger and crew manifest tracking all boarded occupants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleManifest {
    pub vehicle_id: VehicleID,
    pub passenger_capacity: usize,
    pub passengers: Vec<PersonID>,
    pub crew: Vec<(PersonID, StaffRole)>,
}

impl VehicleManifest {
    pub fn new(vehicle_id: VehicleID, passenger_capacity: usize) -> Self {
        Self {
            vehicle_id,
            passenger_capacity,
            passengers: Vec::new(),
            crew: Vec::new(),
        }
    }

    /// Number of available seats remaining for passengers.
    pub fn available_seats(&self) -> usize {
        self.passenger_capacity.saturating_sub(self.passengers.len())
    }

    /// True if all passenger seats are occupied.
    pub fn is_full(&self) -> bool {
        self.passengers.len() >= self.passenger_capacity
    }

    /// Add a passenger to the manifest.
    pub fn add_passenger(&mut self, person_id: PersonID) -> bool {
        if self.is_full() {
            return false;
        }
        self.passengers.push(person_id);
        true
    }

    /// Remove a passenger from the manifest (upon alighting).
    pub fn remove_passenger(&mut self, person_id: PersonID) -> bool {
        if let Some(pos) = self.passengers.iter().position(|&p| p == person_id) {
            self.passengers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Assign a crew member to the vessel.
    pub fn add_crew(&mut self, person_id: PersonID, role: StaffRole) {
        self.crew.push((person_id, role));
    }

    /// Returns true if the vessel has a certified Captain assigned.
    pub fn has_captain(&self) -> bool {
        self.crew.iter().any(|(_, r)| *r == StaffRole::Captain)
    }
}
