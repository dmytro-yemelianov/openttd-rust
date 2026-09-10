use serde::{Deserialize, Serialize};
use transport_types::{CargoType, PersonID, StationID, Ticks, TileIndex, VehicleID};

/// Events that can be emitted by the simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// A vehicle has moved to a new tile
    VehicleMoved {
        vehicle_id: VehicleID,
        old_tile: TileIndex,
        new_tile: TileIndex,
        tick: Ticks,
    },
    /// Cargo has been loaded at a station
    CargoLoaded {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: transport_types::unit::CargoAmount,
        tick: Ticks,
    },
    /// Cargo has been unloaded at a station
    CargoUnloaded {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: transport_types::unit::CargoAmount,
        tick: Ticks,
    },
    /// A person has created a boarding request
    BoardingRequestCreated {
        request_id: u64,
        person_id: PersonID,
        vehicle_id: VehicleID,
        station_id: StationID,
        tick: Ticks,
    },
    /// A boarding request has been approved
    BoardingRequestApproved {
        request_id: u64,
        evidence_id: u64,
        tick: Ticks,
    },
    /// A boarding request has been denied
    BoardingRequestDenied { request_id: u64, tick: Ticks },
    /// A person has boarded a vehicle
    PersonBoarded {
        person_id: PersonID,
        vehicle_id: VehicleID,
        tick: Ticks,
    },
    /// A person has alighted from a vehicle
    PersonAlighted {
        person_id: PersonID,
        vehicle_id: VehicleID,
        tick: Ticks,
    }, // TODO: more events (journey started, assignment completed, etc.)
}
