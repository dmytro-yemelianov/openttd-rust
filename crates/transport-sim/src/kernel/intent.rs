use super::event::KernelEvent;
use transport_types::{CargoAmount, CargoType, CompanyID, Money, StationID, TileIndex, VehicleID};

/// Transactional intent submitted by a driver to be validated and committed during Phase::Commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelIntent {
    /// Credit revenue to a company treasury
    CreditRevenue {
        company_id: CompanyID,
        amount: Money,
    },
    /// Deduct expense/cost from a company treasury
    DeductCost {
        company_id: CompanyID,
        amount: Money,
    },
    /// Update vehicle position to a target tile
    MoveVehicle {
        vehicle_id: VehicleID,
        to_tile: TileIndex,
    },
    /// Transfer cargo between a vehicle and a station
    TransferCargo {
        vehicle_id: VehicleID,
        station_id: StationID,
        cargo_type: CargoType,
        amount: CargoAmount,
        is_load: bool,
    },
    /// Emit an event on the egress event bus
    EmitEvent(KernelEvent),
}
