use serde::{Deserialize, Serialize};

/// Strongly-typed identifier for registered subsystem services / drivers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceId {
    Movement,
    Logistics,
    People,
    Economy,
    Pathfinding,
    Storage,
    Api,
    Custom(u16),
}

/// Execution priority order for deterministic phase dispatch.
/// Lower numerical value executes earlier (Critical = 0, High = 10, Normal = 20, Low = 30).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ServicePriority(pub u8);

impl ServicePriority {
    pub const CRITICAL: Self = Self(0);
    pub const HIGH: Self = Self(10);
    pub const NORMAL: Self = Self(20);
    pub const LOW: Self = Self(30);
}

impl Default for ServicePriority {
    fn default() -> Self {
        Self::NORMAL
    }
}
