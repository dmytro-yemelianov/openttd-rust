use serde::{Deserialize, Serialize};

/// Types of terrain/infrastructure at a tile location
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TileKind {
    Clear,
    Grass,
    Water,
    Rail,
    Road,
    House,
    Trees,
    Station,
    Industry,
    TunnelBridge,
    OilRig,
    // Add more as needed
}

/// Kind of vehicle propulsion
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum VehicleKind {
    Train,
    RoadVehicle,
    Ship,
    Aircraft,
    // Special objects
    Effect,
    Disaster,
}

/// Primary cargo classification
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CargoClass {
    /// Passengers, mail, valuables
    Passenger,
    /// Bulk goods like coal, ore, grain
    Bulk,
    /// Liquid goods like oil, water
    Liquid,
    /// Refrigerated goods
    Refrigerated,
    /// Valuables, goods needing special handling
    Valuable,
    /// Other miscellaneous cargo
    Other,
}

/// Cargo payment modifiers
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CargoPaymentFlags {
    /// Standard payment
    None = 0,
    /// Payment per mile/km
    Distance = 1 << 0,
    /// Payment increases with time in transit
    Time = 1 << 1,
    /// Payment for urgency
    Urgency = 1 << 2,
    /// Special cargo type
    Special = 1 << 3,
}

use crate::id::StationID;

/// Execution state of a vehicle executing its order schedule
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum VehicleState {
    /// Idle (e.g. no orders, or unassigned)
    Idle,
    /// Traveling towards current destination
    Traveling,
    /// Waiting at a station or tile for a specified remaining duration (ticks)
    Waiting(u16),
    /// Loading/unloading cargo at a station
    Loading,
}

impl Default for VehicleState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Order types for vehicle schedules
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Go to specific station (and optionally wait for cargo)
    GoToStation {
        station_id: StationID,
        conditions: Option<WaitConditions>,
    },
    /// Go to specific coordinate (water/air)
    GoToTile {
        x: i16,
        y: i16,
    },
    /// Wait for specific amount of time
    WaitTime(u16), // ticks
    /// Wait until specific date
    WaitDate(u16, u8), // month, day
    /// Go to oil rig (if applicable)
    GoToOilRig,
    /// Visit tin mine (if applicable)
    VisitTinMine,
    /// No order (end of list)
    NoOrder,
}

/// Conditions for waiting at a station for cargo
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WaitConditions {
    /// Minimum cargo amount to wait for
    pub min_amount: u16,
    /// Specific cargo types to wait for (None = any)
    pub cargo_types: Option<Vec<super::id::CargoType>>,
    /// Maximum time to wait (ticks)
    pub max_ticks: u32,
}

/// Link graph node type (for cargo routing)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum LinkGraphNodeType {
    Station,
    /// Proxy node for industries/towns not directly connected
    Proxy,
}

/// Direction of travel on a tile
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TileDirection {
    North,
    East,
    South,
    West,
    /// For diagonal movement (if supported)
    Northeast,
    Southeast,
    Southwest,
    Northwest,
}

/// Signal types for rail
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SignalType {
    None,
    OneWay,
    TwoWay,
    PathSignal,
    ComboSignal,
    /// Custom signal from NewGRF
    Custom(u16),
}

/// Road types
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RoadType {
    None,
    Concrete,
    Asphalt,
    Gravel,
    /// Custom road from NewGRF
    Custom(u16),
}

/// Person activity state
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PersonActivity {
    /// At home/work location
    AtLocation,
    /// Walking to/from transport
    Walking,
    /// Aboard a vehicle
    AboardVehicle,
    /// Waiting for transport
    Waiting,
    /// At destination
    AtDestination,
    /// Other activity
    Other,
}

/// Journey leg type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum JourneyLegType {
    /// Walking segment
    Walk,
    /// Transport segment (aboard vehicle)
    Transport,
    /// Waiting for transport
    Wait,
    /// Transfer between vehicles
    Transfer,
}

/// Authorization evidence type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AuthorizationEvidenceType {
    /// Ticket or pass
    Ticket,
    /// Company employee ID
    EmployeeID,
    /// Special authorization document
    Document,
    /// Biometric verification
    Biometric,
}