pub use crate::definitions::{GoodsEntry, Station};
use serde::{Deserialize, Serialize};
use transport_types::enum_::{VehicleKind, VehicleState};
use transport_types::unit::{CargoAmount, Money};
use transport_types::{
    CargoType, CompanyID, EngineID, OrderIndex, OrderListID, TileIndex, VehicleID,
};

/// A transport company
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Company {
    pub id: CompanyID,
    pub name: String,
    pub money: Money,
    pub color: u32, // RGB color
                    // TODO: more fields as needed
}

impl Company {
    pub fn new(id: CompanyID, name: String, money: Money, color: u32) -> Self {
        Self {
            id,
            name,
            money,
            color,
        }
    }
}

/// A vehicle in the simulation
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: VehicleID,
    pub company_id: CompanyID,
    pub engine_id: EngineID, // Reference to engine definition
    pub kind: VehicleKind,
    pub position: TileIndex, // Current tile position
    pub velocity: i16,       // In tiles per tick? Actually, we might need sub-tile position.
    // For simplicity, we'll use tile position and assume movement between tiles takes multiple ticks.
    // We'll need a more detailed position for movement within a tile.
    // Let's use a separate position within the tile for now.
    pub sub_tile_position: (i16, i16), // Offset within the tile (0..15 for each axis, assuming 16x16 sub-tiles)
    pub current_order: Option<OrderIndex>, // Index in the order list
    #[serde(default)]
    pub state: VehicleState, // Execution state of vehicle in order schedule
    pub cargo: Vec<(CargoType, CargoAmount)>, // Cargo carried
    pub orders: OrderListID,           // The order list this vehicle is following
                                       // TODO: more fields (state, age, reliability, etc.)
}

impl Vehicle {
    pub fn new(
        id: VehicleID,
        company_id: CompanyID,
        engine_id: EngineID,
        kind: VehicleKind,
        position: TileIndex,
        orders: OrderListID,
    ) -> Self {
        Self {
            id,
            company_id,
            engine_id,
            kind,
            position,
            velocity: 0,
            sub_tile_position: (0, 0),
            current_order: None,
            state: VehicleState::Idle,
            cargo: Vec::new(),
            orders,
        }
    }
}
