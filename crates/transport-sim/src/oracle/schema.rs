use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use transport_types::enum_::VehicleState;
use transport_types::{CargoAmount, CargoType, CompanyID, Money, StationID, TileIndex, VehicleID};

/// Compatible state projection of a single vehicle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleTrace {
    pub position: TileIndex,
    pub state: VehicleState,
    pub cargo: Vec<(CargoType, CargoAmount)>,
}

/// Normalized canonical state frame at a discrete tick for oracle comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleTraceFrame {
    pub tick: u32,
    pub companies: BTreeMap<CompanyID, Money>,
    pub vehicles: BTreeMap<VehicleID, VehicleTrace>,
    pub stations: BTreeMap<StationID, BTreeMap<CargoType, CargoAmount>>,
}
