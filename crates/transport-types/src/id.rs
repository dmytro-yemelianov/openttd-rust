use serde::{Deserialize, Serialize};
use std::fmt;

/// A strongly-typed identifier for transport simulation entities.
/// 
/// Each ID type is a newtype wrapper around an integer to prevent
/// mixing different ID types while maintaining performance.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CompanyID(pub u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct VehicleID(pub u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StationID(pub u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OrderListID(pub u32);

/// Index within an order list
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OrderIndex(pub u16);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CargoType(pub u16);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CargoPacketID(pub u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PersonID(pub u64); // Using u64 for external ID compatibility

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct JourneyID(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AssignmentID(pub u64);

/// A tile index in the simulation map
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TileIndex {
    pub x: u16,
    pub y: u16,
}

impl TileIndex {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub const fn max_value() -> Self {
        Self { x: u16::MAX, y: u16::MAX }
    }
}

impl fmt::Display for TileIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

/// Invalid ID sentinel values
pub mod invalid {
    use super::*;

    pub const INVALID_COMPANY_ID: CompanyID = CompanyID(u32::MAX);
    pub const INVALID_VEHICLE_ID: VehicleID = VehicleID(u32::MAX);
    pub const INVALID_STATION_ID: StationID = StationID(u32::MAX);
    pub const INVALID_ORDER_LIST_ID: OrderListID = OrderListID(u32::MAX);
    pub const INVALID_CARGO_TYPE: CargoType = CargoType(u16::MAX);
    pub const INVALID_CARGO_PACKET_ID: CargoPacketID = CargoPacketID(u32::MAX);
    pub const INVALID_PERSON_ID: PersonID = PersonID(u64::MAX);
    pub const INVALID_JOURNEY_ID: JourneyID = JourneyID(u64::MAX);
    pub const INVALID_ASSIGNMENT_ID: AssignmentID = AssignmentID(u64::MAX);
}

/// Extension trait for checking ID validity
pub trait IsValid {
    fn is_valid(&self) -> bool;
}

impl IsValid for CompanyID {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl IsValid for VehicleID {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl IsValid for StationID {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl IsValid for OrderListID {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl IsValid for CargoType {
    fn is_valid(&self) -> bool {
        self.0 != u16::MAX
    }
}

impl IsValid for CargoPacketID {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl IsValid for PersonID {
    fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl IsValid for JourneyID {
    fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl IsValid for AssignmentID {
    fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl IsValid for TileIndex {
    fn is_valid(&self) -> bool {
        self.x != u16::MAX && self.y != u16::MAX
    }
}