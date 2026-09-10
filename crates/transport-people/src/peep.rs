use serde::{Deserialize, Serialize};
use transport_types::{Money, PersonID, StationID, TileIndex, VehicleID};

/// Staff or passenger role adapted from OpenRCT2 staff mechanics.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum StaffRole {
    Passenger,
    Captain,
    Deckhand,
    Engineer,
}

impl Default for StaffRole {
    fn default() -> Self {
        Self::Passenger
    }
}

/// State machine for a Peep traveling across the transport network.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PeepState {
    /// At home / origin location
    AtOrigin,
    /// Physically walking towards station turnstiles
    WalkingToStation { target_station: StationID },
    /// Waiting in FIFO queue line at a dock entrance turnstile
    InStationQueue { station_id: StationID },
    /// Passing through turnstile / stepping across gangway
    Boarding { vehicle_id: VehicleID },
    /// Aboard a vehicle in transit
    AboardVehicle { vehicle_id: VehicleID },
    /// Alighting from vehicle at destination dock
    Alighting { station_id: StationID },
    /// Journey completed successfully
    Completed,
}

impl Default for PeepState {
    fn default() -> Self {
        Self::AtOrigin
    }
}

/// A compact, cache-friendly individual person agent adapted from OpenRCT2.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Peep {
    pub id: PersonID,
    pub origin_tile: TileIndex,
    pub current_tile: TileIndex,
    pub destination_station: StationID,
    pub state: PeepState,
    pub sub_tile: (u8, u8), // Sub-tile coordinate (0..15, 0..15)
    pub wallet: Money,
    pub fare_paid: Money,
    pub wait_ticks: u16,
    pub role: StaffRole,
}

impl Peep {
    pub fn new(id: PersonID, origin_tile: TileIndex, destination_station: StationID, wallet: Money) -> Self {
        Self {
            id,
            origin_tile,
            current_tile: origin_tile,
            destination_station,
            state: PeepState::AtOrigin,
            sub_tile: (8, 8),
            wallet,
            fare_paid: Money(0),
            wait_ticks: 0,
            role: StaffRole::Passenger,
        }
    }

    pub fn with_role(mut self, role: StaffRole) -> Self {
        self.role = role;
        self
    }
}
