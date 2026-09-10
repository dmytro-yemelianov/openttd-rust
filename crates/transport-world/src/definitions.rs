use serde::{Deserialize, Serialize};
use transport_types::enum_::{CargoClass, VehicleKind};
use transport_types::unit::{CargoAmount, Money, Speed};
use transport_types::{CargoType, CompanyID, EngineID, StationID, TileIndex};

/// Definition of an engine (vehicle model)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Engine {
    pub id: EngineID,
    pub name: String,
    pub kind: VehicleKind,
    pub power: u16,                     // In horsepower?
    pub tractive_effort: u16,           // In pounds?
    pub weight: u16,                    // In tons?
    pub speed: Speed,                   // Max speed
    pub cost: Money,                    // Purchase cost
    pub running_cost: Money,            // Cost per tick
    pub cargo_classes: Vec<CargoClass>, // What cargo classes it can carry
    pub introduction_date: u16,         // Year introduced
    pub retirement_date: Option<u16>,   // Year retired
                                        // TODO: more fields (reliability, etc.)
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EngineID,
        name: String,
        kind: VehicleKind,
        power: u16,
        tractive_effort: u16,
        weight: u16,
        speed: Speed,
        cost: Money,
        running_cost: Money,
        cargo_classes: Vec<CargoClass>,
        introduction_date: u16,
        retirement_date: Option<u16>,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            power,
            tractive_effort,
            weight,
            speed,
            cost,
            running_cost,
            cargo_classes,
            introduction_date,
            retirement_date,
        }
    }
}

/// Definition of a cargo type
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Cargo {
    pub id: CargoType,
    pub name: String,
    pub class: CargoClass,
    pub payment_factor: u16,           // Base payment per unit per tile?
    pub payment_factor_percentage: u8, // Percentage of payment factor?
    pub penalty_factor: u8,            // Penalty for late delivery?
                                       // TODO: more fields (weight, etc.)
}

impl Cargo {
    pub fn new(id: CargoType, name: String, class: CargoClass) -> Self {
        Self {
            id,
            name,
            class,
            payment_factor: 0,
            payment_factor_percentage: 0,
            penalty_factor: 0,
        }
    }
}

/// Goods entry for a specific cargo type at a station
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GoodsEntry {
    pub cargo_type: CargoType,
    pub amount: CargoAmount, // Amount of cargo waiting at the station
    pub rating: u8,          // Rating (0-255) for this cargo type at this station
    pub income_since_last_visit: i32, // Income from this cargo type since last vehicle visit
    pub delivered_since_last_visit: CargoAmount, // Amount delivered since last visit
                             // TODO: more fields (acceptance, etc.)
}

impl GoodsEntry {
    pub fn new(cargo_type: CargoType) -> Self {
        Self {
            cargo_type,
            amount: CargoAmount(0),
            rating: 0,
            income_since_last_visit: 0,
            delivered_since_last_visit: CargoAmount(0),
        }
    }
}

/// Station (or dock) where cargo can be loaded/unloaded
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Station {
    pub id: StationID,
    pub company_id: Option<CompanyID>, // None if unowned
    pub name: String,
    pub tiles: Vec<TileIndex>, // Tiles that make up the station
    #[serde(default, alias = "rating")]
    pub operator_rating: u8,
    pub goods: Vec<GoodsEntry>, // One per cargo type
                                // TODO: more fields (acceptance, etc.)
}

impl Station {
    pub fn new(id: StationID, name: String, tiles: Vec<TileIndex>) -> Self {
        Self {
            id,
            company_id: None,
            name,
            tiles,
            operator_rating: 0,
            goods: Vec::new(),
        }
    }
}
