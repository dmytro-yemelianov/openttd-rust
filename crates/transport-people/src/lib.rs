//! # Transport simulation people extension
//!
//! This crate contains the extension for simulating individual people:
//! - Person entity with location and activity
//! - Journeys and assignments
//! - Boarding and authorization logic

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use transport_types::{PersonID, JourneyID, AssignmentID, VehicleID, StationID, TileIndex};
use transport_types::unit::{Ticks, Money};
use transport_types::enum_::{PersonActivity, JourneyLegType, AuthorizationEvidenceType};

/// A person in the simulation
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonID,
    pub name: String,
    pub current_location: Location,
    pub activity: PersonActivity,
    pub assigned_journey: Option<JourneyID>,
    pub money: Money, // Personal money
    // TODO: more fields (home location, workplace, etc.)
}

impl Person {
    pub fn new(id: PersonID, name: String) -> Self {
        Self {
            id,
            name,
            current_location: Location::AtLocation { tile_id: None }, // Start at home/work?
            activity: PersonActivity::AtLocation,
            assigned_journey: None,
            money: Money(0),
        }
    }
}

/// Where a person can be located
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Location {
    /// At a specific location (home, work, etc.) - tile_id is optional for off-map locations
    AtLocation { tile_id: Option<TileIndex> },
    /// Walking between two points (from tile to tile)
    Walking {
        from: TileIndex,
        to: TileIndex,
        progress: u8, // 0-100 percentage
    },
    /// Aboard a vehicle
    AboardVehicle {
        vehicle_id: VehicleID,
        // The journey leg they are on (if part of a journey)
        journey_leg: Option<u32>, // Index of the leg in their journey
    },
    /// Waiting for transport at a station or stop
    WaitingAtStop {
        station_id: StationID,
        // Are they waiting for a specific vehicle or just any?
        waiting_for_vehicle: Option<VehicleID>,
    },
    /// Outside the simulated world (e.g., in another city not modeled)
    OutsideWorld,
}

/// A journey that a person undertakes
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Journey {
    pub id: JourneyID,
    pub person_id: PersonID,
    pub origin: TileIndex, // Starting location
    pub destination: TileIndex, // Ending location
    pub purpose: JourneyPurpose, // Why they are traveling
    pub legs: Vec<JourneyLeg>, // Ordered list of legs
    pub current_leg_index: usize, // Which leg they are currently on
    pub state: JourneyState, // Current state of the journey
    pub start_time: Ticks, // When the journey started
    pub end_time: Option<Ticks>, // When the journey ended (if completed)
    // TODO: more fields (budget, time constraints, etc.)
}

impl Journey {
    pub fn new(
        id: JourneyID,
        person_id: PersonID,
        origin: TileIndex,
        destination: TileIndex,
        purpose: JourneyPurpose,
    ) -> Self {
        Self {
            id,
            person_id,
            origin,
            destination,
            purpose,
            legs: Vec::new(),
            current_leg_index: 0,
            state: JourneyState::Planned,
            start_time: Ticks(0),
            end_time: None,
        }
    }
}

/// The purpose of a journey
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum JourneyPurpose {
    Commute,
    Business,
    Leisure,
    Other,
}

/// A leg of a journey
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct JourneyLeg {
    pub leg_type: JourneyLegType,
    pub start_location: TileIndex, // Starting point of the leg
    pub end_location: TileIndex, // Ending point of the leg
    pub vehicle_id: Option<VehicleID>, // For transport legs, the vehicle used
    pub waiting_time: Ticks, // Time spent waiting at the start of the leg
    pub travel_time: Ticks, // Actual travel time for the leg
    // TODO: more fields (cost, etc.)
}

/// The current state of a journey
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum JourneyState {
    Planned, // Journey is planned but not started
    Ongoing, // Journey is in progress
    Completed, // Journey has finished
    Cancelled, // Journey was cancelled
    Failed, // Journey failed (e.g., missed connection)
}

/// An assignment (e.g., a job) that a person has
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Assignment {
    pub id: AssignmentID,
    pub person_id: PersonID,
    pub workplace: TileIndex, // Tile of the workplace
    pub role: String, // Job role
    pub schedule: Vec<(Ticks, Ticks)>, // List of (start, end) time ticks when they should be at work
    pub current_shift_index: usize, // Which shift they are currently on
    pub money: Money, // Wages earned from this assignment
    // TODO: more fields (salary, benefits, etc.)
}

impl Assignment {
    pub fn new(id: AssignmentID, person_id: PersonID, workplace: TileIndex, role: String) -> Self {
        Self {
            id,
            person_id,
            workplace,
            role,
            schedule: Vec::new(),
            current_shift_index: 0,
            money: Money(0),
        }
    }
}

/// Evidence of authorization for boarding or crossing
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AuthorizationEvidence {
    pub evidence_id: u64, // Unique ID for this evidence
    pub request_id: u64, // ID of the boarding request this evidence responds to
    pub evidence_type: AuthorizationEvidenceType,
    pub valid_from: Ticks, // Tick from which this evidence is valid
    pub valid_until: Ticks, // Tick until which this evidence is valid
    pub issuer: String, // Who issued the evidence (e.g., company ID or "Grasida")
    // TODO: more fields (signature, etc.)
}

/// A boarding request from a person to board a vehicle
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BoardingRequest {
    pub request_id: u64, // Unique ID for this request
    pub person_id: PersonID,
    pub vehicle_id: VehicleID,
    pub station_id: StationID, // Where they are trying to board
    pub timestamp: Ticks, // When the request was made
    pub state: BoardingRequestState, // Current state of the request
    // TODO: more fields (intended crossing, etc.)
}

/// The state of a boarding request
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BoardingRequestState {
    Pending, // Requested, waiting for authorization
    Approved, // Authorization evidence provided and valid
    Denied, // Authorization denied or evidence invalid
    TimedOut, // Request took too long and was cancelled
    Boarded, // Person successfully boarded
}

/// The simulator extension for people
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeopleSimulator {
    pub people: HashMap<PersonID, Person>,
    pub journeys: HashMap<JourneyID, Journey>,
    pub assignments: HashMap<AssignmentID, Assignment>,
    pub boarding_requests: HashMap<u64, BoardingRequest>, // Keyed by request ID
    pub authorization_evidence: HashMap<u64, AuthorizationEvidence>, // Keyed by evidence ID
    // TODO: more fields (event queues, etc.)
}

impl Default for PeopleSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PeopleSimulator {
    pub fn new() -> Self {
        Self {
            people: HashMap::new(),
            journeys: HashMap::new(),
            assignments: HashMap::new(),
            boarding_requests: HashMap::new(),
            authorization_evidence: HashMap::new(),
        }
    }

    /// Add a new person
    pub fn add_person(&mut self, person: Person) -> PersonID {
        let id = person.id;
        self.people.insert(id, person);
        id
    }

    // TODO: more methods for managing journeys, assignments, boarding, etc.
}