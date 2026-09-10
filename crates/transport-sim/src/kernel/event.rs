use serde::{Deserialize, Serialize};
use transport_types::{CargoAmount, CargoType, StationID, Ticks, TileIndex, VehicleID};

/// Observability events emitted on the egress phase of the microkernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelEvent {
    VehicleMoved {
        vehicle_id: VehicleID,
        old_tile: TileIndex,
        new_tile: TileIndex,
        tick: Ticks,
    },
    CargoTransferred {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: CargoAmount,
        is_load: bool,
        tick: Ticks,
    },
}
