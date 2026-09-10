use serde::{Deserialize, Serialize};
use std::cmp::{Ord, PartialOrd};
use std::fmt;

/// A strongly-typed identifier for transport simulation entities.
/// 
/// Each ID type is a newtype wrapper around an integer to prevent
/// mixing different ID types while maintaining performance.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CompanyID(pub u32);
impl CompanyID {
    pub const INVALID: Self = Self(u32::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct VehicleID(pub u32);
impl VehicleID {
    pub const INVALID: Self = Self(u32::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct StationID(pub u32);
impl StationID {
    pub const INVALID: Self = Self(u32::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct OrderListID(pub u32);
impl OrderListID {
    pub const INVALID: Self = Self(u32::MAX);
}

/// Index within an order list
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct OrderIndex(pub u16);
impl OrderIndex {
    pub const INVALID: Self = Self(u16::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CargoType(pub u16);
impl CargoType {
    pub const INVALID: Self = Self(u16::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CargoPacketID(pub u32);
impl CargoPacketID {
    pub const INVALID: Self = Self(u32::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PersonID(pub u64); // Using u64 for external ID compatibility
impl PersonID {
    pub const INVALID: Self = Self(u64::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct JourneyID(pub u64);
impl JourneyID {
    pub const INVALID: Self = Self(u64::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AssignmentID(pub u64);
impl AssignmentID {
    pub const INVALID: Self = Self(u64::MAX);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct EngineID(pub u32);
impl EngineID {
    pub const INVALID: Self = Self(u32::MAX);
}

/// A tile index in the simulation map
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
    pub const INVALID: Self = Self { x: u16::MAX, y: u16::MAX };
}

impl fmt::Display for TileIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }

}

/// Extension trait for checking ID validity
pub trait IsValid {
    fn is_valid(&self) -> bool;
}

impl IsValid for CompanyID {
    fn is_valid(&self) -> bool {
        self.0 != CompanyID::INVALID.0
    }
}

impl IsValid for VehicleID {
    fn is_valid(&self) -> bool {
        self.0 != VehicleID::INVALID.0
    }
}

impl IsValid for StationID {
    fn is_valid(&self) -> bool {
        self.0 != StationID::INVALID.0
    }
}

impl IsValid for OrderListID {
    fn is_valid(&self) -> bool {
        self.0 != OrderListID::INVALID.0
    }
}

impl IsValid for OrderIndex {
    fn is_valid(&self) -> bool {
        self.0 != OrderIndex::INVALID.0
    }
}

impl IsValid for CargoType {
    fn is_valid(&self) -> bool {
        self.0 != CargoType::INVALID.0
    }
}

impl IsValid for CargoPacketID {
    fn is_valid(&self) -> bool {
        self.0 != CargoPacketID::INVALID.0
    }
}

impl IsValid for PersonID {
    fn is_valid(&self) -> bool {
        self.0 != PersonID::INVALID.0
    }
}

impl IsValid for JourneyID {
    fn is_valid(&self) -> bool {
        self.0 != JourneyID::INVALID.0
    }
}

impl IsValid for AssignmentID {
    fn is_valid(&self) -> bool {
        self.0 != AssignmentID::INVALID.0
    }
}

impl IsValid for EngineID {
    fn is_valid(&self) -> bool {
        self.0 != EngineID::INVALID.0
    }
}

impl IsValid for TileIndex {
    fn is_valid(&self) -> bool {
        self.x != TileIndex::INVALID.x && self.y != TileIndex::INVALID.y
    }
}