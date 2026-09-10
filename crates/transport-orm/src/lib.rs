#![doc = "Object-relational mapping for transport simulation"]
//!
//! This crate provides serialization and deserialization of the simulation state:
//! - Versioned envelope containing world, people, pending commands, counters, and rulesets
//! - Atomic generational persistence with temporary flush and atomic rename
//! - Retention of previous valid generations (.bak)
//! - State invariant validation on restore
//! - Derived spatial index rebuilding on load
//! - Legacy migration for station ratings

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use transport_people::{Location, PeopleSimulator};
use transport_sim::{CommandOutcome, QueuedCommand, Simulator, World};
use transport_types::StationID;

/// Current snapshot format version
pub const CURRENT_SNAPSHOT_VERSION: u32 = 1;

/// Current ruleset version
pub const CURRENT_RULESET_VERSION: &str = "0.1.0";

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Error types for the ORM
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unsupported snapshot format version: {actual} (expected {expected})")]
    UnsupportedVersion { actual: u32, expected: u32 },
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Invalid station rating for station {0:?}: {1} (must be <= 100)")]
    InvalidStationRating(StationID, u8),
    #[error("Map validation error: {0}")]
    MapValidation(#[from] transport_world::map::MapError),
}

/// Result type for ORM operations
pub type Result<T> = std::result::Result<T, Error>;

/// Versioned snapshot envelope containing complete simulation state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEnvelope {
    pub format_version: u32,
    pub ruleset_version: String,
    pub prng_state: Option<u64>,
    pub world: World,
    pub people: PeopleSimulator,
    pub command_queue: VecDeque<QueuedCommand>,
    pub next_submission_id: u64,
    pub outcomes: HashMap<u64, CommandOutcome>,
    pub execution_order_next: u64,
}

impl SnapshotEnvelope {
    /// Create an envelope from a live Simulator and PeopleSimulator
    pub fn from_simulator(simulator: &Simulator, people: &PeopleSimulator) -> Self {
        Self {
            format_version: CURRENT_SNAPSHOT_VERSION,
            ruleset_version: CURRENT_RULESET_VERSION.to_string(),
            prng_state: None,
            world: simulator.world.clone(),
            people: people.clone(),
            command_queue: simulator.command_queue.clone(),
            next_submission_id: simulator.next_submission_id,
            outcomes: simulator.outcomes.clone(),
            execution_order_next: simulator.execution_order_next,
        }
    }

    /// Create an envelope from just World and PeopleSimulator
    pub fn from_world_and_people(world: &World, people: &PeopleSimulator) -> Self {
        Self {
            format_version: CURRENT_SNAPSHOT_VERSION,
            ruleset_version: CURRENT_RULESET_VERSION.to_string(),
            prng_state: None,
            world: world.clone(),
            people: people.clone(),
            command_queue: VecDeque::new(),
            next_submission_id: 0,
            outcomes: HashMap::new(),
            execution_order_next: 0,
        }
    }

    /// Restore live Simulator and PeopleSimulator from the envelope
    pub fn restore_simulator(&self) -> (Simulator, PeopleSimulator) {
        let mut sim = Simulator {
            world: self.world.clone(),
            command_queue: self.command_queue.clone(),
            next_submission_id: self.next_submission_id,
            outcomes: self.outcomes.clone(),
            execution_order_next: self.execution_order_next,
            vehicle_order_buffer: Vec::new(),
        };
        sim.world.rebuild_derived_indexes();
        (sim, self.people.clone())
    }
}

/// Helper to resolve target snapshot path (file or directory)
pub fn resolve_snapshot_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if p.is_dir() || p.extension().is_none() {
        p.join("snapshot.json")
    } else {
        p.to_path_buf()
    }
}

/// Save the world state to a JSON file
pub fn save_world<P: AsRef<Path>>(world: &World, path: P) -> Result<()> {
    let json = serde_json::to_string_pretty(world)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load the world state from a JSON file
pub fn load_world<P: AsRef<Path>>(path: P) -> Result<World> {
    let json = fs::read_to_string(path)?;
    let mut world: World = serde_json::from_str(&json)?;
    world.map.validate()?;
    world.rebuild_derived_indexes();
    Ok(world)
}

/// Save the people simulator state to a JSON file
pub fn save_people<P: AsRef<Path>>(people: &PeopleSimulator, path: P) -> Result<()> {
    let json = serde_json::to_string_pretty(people)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load the people simulator state from a JSON file
pub fn load_people<P: AsRef<Path>>(path: P) -> Result<PeopleSimulator> {
    let json = fs::read_to_string(path)?;
    let people: PeopleSimulator = serde_json::from_str(&json)?;
    Ok(people)
}

/// Save a versioned snapshot envelope using atomic generational persistence
pub fn save_snapshot_envelope<P: AsRef<Path>>(envelope: &SnapshotEnvelope, path: P) -> Result<()> {
    validate_snapshot(envelope)?;

    let target_path = resolve_snapshot_path(path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(envelope)?;
    let counter = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_file_name = format!(
        ".{}.tmp.{}_{}",
        target_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id(),
        counter
    );
    let tmp_path = target_path.with_file_name(tmp_file_name);

    // RAII guard to clean up temp file if writing or syncing fails
    struct TempFileGuard<'a>(&'a Path);
    impl<'a> Drop for TempFileGuard<'a> {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0);
        }
    }
    let guard = TempFileGuard(&tmp_path);

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }

    // Retain previous valid generation if destination exists
    let bak_path = target_path.with_extension("bak");
    if target_path.exists() {
        let _ = fs::copy(&target_path, &bak_path);
    }

    // Atomically replace target path
    fs::rename(&tmp_path, &target_path)?;

    // Disarm cleanup guard
    std::mem::forget(guard);

    Ok(())
}

/// Load a versioned snapshot envelope, with automatic recovery from backup if corrupt
pub fn load_snapshot_envelope<P: AsRef<Path>>(path: P) -> Result<SnapshotEnvelope> {
    let target_path = resolve_snapshot_path(path.as_ref());
    let bak_path = target_path.with_extension("bak");

    // Check for legacy directory format first if target_path does not exist
    if !target_path.exists() {
        let parent = path.as_ref();
        let legacy_world = parent.join("world.json");
        let legacy_people = parent.join("people.json");
        if legacy_world.exists() {
            let world = load_world(&legacy_world)?;
            let people = if legacy_people.exists() {
                load_people(&legacy_people)?
            } else {
                PeopleSimulator::new()
            };
            let env = SnapshotEnvelope::from_world_and_people(&world, &people);
            validate_snapshot(&env)?;
            return Ok(env);
        }
    }

    let content = match fs::read_to_string(&target_path) {
        Ok(c) => c,
        Err(e) => {
            if bak_path.exists() {
                fs::read_to_string(&bak_path)?
            } else {
                return Err(Error::Io(e));
            }
        }
    };

    let mut envelope: SnapshotEnvelope = match serde_json::from_str(&content) {
        Ok(env) => env,
        Err(e) => {
            if bak_path.exists() {
                let bak_content = fs::read_to_string(&bak_path)?;
                serde_json::from_str(&bak_content)?
            } else {
                return Err(Error::Json(e));
            }
        }
    };

    validate_snapshot(&envelope)?;
    envelope.world.rebuild_derived_indexes();
    Ok(envelope)
}

/// Save a simulator and people state to a recoverable snapshot
pub fn save_simulator_snapshot<P: AsRef<Path>>(
    simulator: &Simulator,
    people: &PeopleSimulator,
    path: P,
) -> Result<()> {
    let envelope = SnapshotEnvelope::from_simulator(simulator, people);
    save_snapshot_envelope(&envelope, path)
}

/// Load a simulator and people state from a recoverable snapshot
pub fn load_simulator_snapshot<P: AsRef<Path>>(path: P) -> Result<(Simulator, PeopleSimulator)> {
    let envelope = load_snapshot_envelope(path)?;
    Ok(envelope.restore_simulator())
}

/// Save world and people to a snapshot
pub fn save_snapshot<P: AsRef<Path>>(
    world: &World,
    people: &PeopleSimulator,
    path: P,
) -> Result<()> {
    let envelope = SnapshotEnvelope::from_world_and_people(world, people);
    save_snapshot_envelope(&envelope, path)
}

/// Load world and people from a snapshot
pub fn load_snapshot<P: AsRef<Path>>(path: P) -> Result<(World, PeopleSimulator)> {
    let envelope = load_snapshot_envelope(path)?;
    let (sim, people) = envelope.restore_simulator();
    Ok((sim.world, people))
}

/// Validate all simulation state invariants before restoring
pub fn validate_snapshot(envelope: &SnapshotEnvelope) -> Result<()> {
    if envelope.format_version != CURRENT_SNAPSHOT_VERSION {
        return Err(Error::UnsupportedVersion {
            actual: envelope.format_version,
            expected: CURRENT_SNAPSHOT_VERSION,
        });
    }

    // 1. Validate Map
    envelope.world.map.validate()?;

    // 2. Validate Companies
    for (id, company) in &envelope.world.companies {
        if *id != company.id {
            return Err(Error::Validation(format!(
                "Company ID mismatch: key {:?} != company.id {:?}",
                id, company.id
            )));
        }
    }

    // 3. Validate Stations
    for (id, station) in &envelope.world.stations {
        if *id != station.id {
            return Err(Error::Validation(format!(
                "Station ID mismatch: key {:?} != station.id {:?}",
                id, station.id
            )));
        }
        if station.operator_rating > 100 {
            return Err(Error::InvalidStationRating(*id, station.operator_rating));
        }
        if let Some(company_id) = station.company_id {
            if !envelope.world.companies.is_empty()
                && !envelope.world.companies.contains_key(&company_id)
            {
                return Err(Error::Validation(format!(
                    "Station {id:?} references non-existent company {company_id:?}"
                )));
            }
        }
        for tile in &station.tiles {
            if !envelope.world.map.size().is_valid_index(*tile) {
                return Err(Error::Validation(format!(
                    "Station {id:?} contains out-of-bounds tile {tile:?}"
                )));
            }
        }
    }

    // 4. Validate Vehicles
    for (id, vehicle) in &envelope.world.vehicles {
        if *id != vehicle.id {
            return Err(Error::Validation(format!(
                "Vehicle ID mismatch: key {:?} != vehicle.id {:?}",
                id, vehicle.id
            )));
        }
        if !envelope.world.map.size().is_valid_index(vehicle.position) {
            return Err(Error::Validation(format!(
                "Vehicle {:?} is at out-of-bounds position {:?}",
                id, vehicle.position
            )));
        }
        if !envelope.world.companies.is_empty()
            && !envelope.world.companies.contains_key(&vehicle.company_id)
        {
            return Err(Error::Validation(format!(
                "Vehicle {:?} references non-existent company {:?}",
                id, vehicle.company_id
            )));
        }
        if !envelope.world.order_lists.is_empty()
            && !envelope.world.order_lists.contains_key(&vehicle.orders)
        {
            return Err(Error::Validation(format!(
                "Vehicle {:?} references non-existent order list {:?}",
                id, vehicle.orders
            )));
        }
    }

    // 5. Validate People
    for (id, person) in &envelope.people.people {
        if *id != person.id {
            return Err(Error::Validation(format!(
                "Person ID mismatch: key {:?} != person.id {:?}",
                id, person.id
            )));
        }
        match &person.current_location {
            Location::AtLocation {
                tile_id: Some(tile),
            } => {
                if !envelope.world.map.size().is_valid_index(*tile) {
                    return Err(Error::Validation(format!(
                        "Person {id:?} at out-of-bounds tile {tile:?}"
                    )));
                }
            }
            Location::Walking { from, to, .. } => {
                if !envelope.world.map.size().is_valid_index(*from)
                    || !envelope.world.map.size().is_valid_index(*to)
                {
                    return Err(Error::Validation(format!(
                        "Person {id:?} walking on out-of-bounds tile"
                    )));
                }
            }
            Location::AboardVehicle { vehicle_id, .. } => {
                if !envelope.world.vehicles.is_empty()
                    && !envelope.world.vehicles.contains_key(vehicle_id)
                {
                    return Err(Error::Validation(format!(
                        "Person {id:?} aboard non-existent vehicle {vehicle_id:?}"
                    )));
                }
            }
            Location::WaitingAtStop { station_id, .. } => {
                if !envelope.world.stations.is_empty()
                    && !envelope.world.stations.contains_key(station_id)
                {
                    return Err(Error::Validation(format!(
                        "Person {id:?} waiting at non-existent station {station_id:?}"
                    )));
                }
            }
            _ => {}
        }
    }

    // 6. Validate Command Queue and Monotonic Sequence IDs
    for queued in &envelope.command_queue {
        if queued.submission_id >= envelope.next_submission_id {
            return Err(Error::Validation(format!(
                "Queued command submission_id {} exceeds next_submission_id {}",
                queued.submission_id, envelope.next_submission_id
            )));
        }
    }
    for sub_id in envelope.outcomes.keys() {
        if *sub_id >= envelope.next_submission_id {
            return Err(Error::Validation(format!(
                "Outcome submission_id {} exceeds next_submission_id {}",
                sub_id, envelope.next_submission_id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use transport_sim::Command;
    use transport_types::unit::Money;
    use transport_types::{CompanyID, OrderListID, StationID, TileIndex, VehicleID};
    use transport_world::definitions::Station;
    use transport_world::entities::Vehicle;
    use transport_world::map::MapSize;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("openttd_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_continuous_vs_save_restore_execution_trace() {
        let dir = temp_test_dir("continuous_vs_restore");
        let save_path = dir.join("snapshot.json");

        let mut sim_continuous = Simulator::new(MapSize::new(20, 20));
        let mut sim_restored = Simulator::new(MapSize::new(20, 20));
        let people = PeopleSimulator::new();

        // Enqueue commands on both
        let cmd1 = Command::CreateCompany {
            name: "TransCorp".to_string(),
            money: Money(50000),
            color: 0x00FF00,
        };
        sim_continuous
            .enqueue_command(cmd1.clone(), "test".to_string(), 10)
            .unwrap();
        sim_restored
            .enqueue_command(cmd1, "test".to_string(), 10)
            .unwrap();

        // Run continuous for 100 ticks
        for _ in 0..100 {
            sim_continuous.tick();
        }

        // Run restored for 50 ticks, save, reload, run for 50 more ticks
        for _ in 0..50 {
            sim_restored.tick();
        }
        assert_eq!(sim_restored.world.tick.0, 50);

        save_simulator_snapshot(&sim_restored, &people, &save_path).unwrap();
        let (mut reloaded_sim, reloaded_people) = load_simulator_snapshot(&save_path).unwrap();

        assert_eq!(reloaded_sim.world.tick.0, 50);
        assert_eq!(reloaded_people, people);

        for _ in 50..100 {
            reloaded_sim.tick();
        }
        assert_eq!(reloaded_sim.world.tick.0, 100);

        // Deterministic equality check: continuous run == save/restore run
        assert_eq!(sim_continuous, reloaded_sim);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_interrupted_save_preserves_previous_snapshot() {
        let dir = temp_test_dir("interrupted_save");
        let save_path = dir.join("snapshot.json");

        let mut sim = Simulator::new(MapSize::new(10, 10));
        let people = PeopleSimulator::new();

        for _ in 0..10 {
            sim.tick();
        }
        assert_eq!(sim.world.tick.0, 10);

        // Save valid snapshot at tick 10
        save_simulator_snapshot(&sim, &people, &save_path).unwrap();

        // Now advance to tick 20
        for _ in 0..10 {
            sim.tick();
        }
        assert_eq!(sim.world.tick.0, 20);

        // Attempt to save an envelope with corrupted/invalid format version
        let mut bad_envelope = SnapshotEnvelope::from_simulator(&sim, &people);
        bad_envelope.format_version = 999; // unsupported

        let err = save_snapshot_envelope(&bad_envelope, &save_path);
        assert!(err.is_err(), "Saving bad envelope must fail validation");

        // The original snapshot file must be intact and still load tick 10!
        let (loaded_sim, _) = load_simulator_snapshot(&save_path).unwrap();
        assert_eq!(
            loaded_sim.world.tick.0, 10,
            "Failed save must not corrupt previous valid snapshot"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generational_backup_and_recovery() {
        let dir = temp_test_dir("generational_backup");
        let save_path = dir.join("snapshot.json");
        let bak_path = dir.join("snapshot.bak");

        let mut sim = Simulator::new(MapSize::new(10, 10));
        let people = PeopleSimulator::new();

        // Generation 1 at tick 10
        for _ in 0..10 {
            sim.tick();
        }
        save_simulator_snapshot(&sim, &people, &save_path).unwrap();

        // Generation 2 at tick 25
        for _ in 0..15 {
            sim.tick();
        }
        save_simulator_snapshot(&sim, &people, &save_path).unwrap();

        // Both snapshot.json (gen 2) and snapshot.bak (gen 1) must exist
        assert!(save_path.exists());
        assert!(bak_path.exists());

        // Generation 2 should be at tick 25
        let (gen2_sim, _) = load_simulator_snapshot(&save_path).unwrap();
        assert_eq!(gen2_sim.world.tick.0, 25);

        // Corrupt main snapshot.json
        fs::write(&save_path, "NOT VALID JSON DATA TRUNCATED").unwrap();

        // Loading should recover seamlessly from snapshot.bak (gen 1 at tick 10)
        let (recovered_sim, _) = load_simulator_snapshot(&save_path).unwrap();
        assert_eq!(
            recovered_sim.world.tick.0, 10,
            "Corrupt snapshot must recover from generation backup"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_reject_unsupported_version() {
        let sim = Simulator::new(MapSize::new(10, 10));
        let people = PeopleSimulator::new();
        let mut envelope = SnapshotEnvelope::from_simulator(&sim, &people);
        envelope.format_version = 2; // future unsupported version

        let result = validate_snapshot(&envelope);
        assert!(matches!(
            result,
            Err(Error::UnsupportedVersion {
                actual: 2,
                expected: CURRENT_SNAPSHOT_VERSION
            })
        ));
    }

    #[test]
    fn test_reject_invalid_station_rating() {
        let mut sim = Simulator::new(MapSize::new(10, 10));
        let people = PeopleSimulator::new();

        let mut station = Station::new(
            StationID(1),
            "Harbor".to_string(),
            vec![TileIndex::new(2, 2)],
        );
        station.operator_rating = 150; // invalid rating (> 100)
        sim.world.stations.insert(StationID(1), station);

        let envelope = SnapshotEnvelope::from_simulator(&sim, &people);
        let result = validate_snapshot(&envelope);
        assert!(matches!(
            result,
            Err(Error::InvalidStationRating(StationID(1), 150))
        ));
    }

    #[test]
    fn test_station_rating_migration() {
        // Old JSON format with "rating" instead of "operator_rating"
        let legacy_json = r#"{
            "id": 1,
            "company_id": null,
            "name": "Old Pier",
            "tiles": [{"x": 1, "y": 1}],
            "rating": 75,
            "goods": []
        }"#;

        let station: Station = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(
            station.operator_rating, 75,
            "Legacy rating field must migrate into operator_rating"
        );
    }

    #[test]
    fn test_derived_index_rebuilt_on_restore() {
        let dir = temp_test_dir("derived_index");
        let save_path = dir.join("snapshot.json");

        let mut sim = Simulator::new(MapSize::new(10, 10));
        let people = PeopleSimulator::new();

        let station = Station::new(
            StationID(42),
            "Central Docks".to_string(),
            vec![TileIndex::new(3, 4), TileIndex::new(3, 5)],
        );
        sim.world.stations.insert(StationID(42), station);
        sim.world.rebuild_derived_indexes();

        // Verify index before save
        assert_eq!(
            sim.world
                .get_station_at_tile(&TileIndex::new(3, 4))
                .map(|s| s.id),
            Some(StationID(42))
        );

        save_simulator_snapshot(&sim, &people, &save_path).unwrap();

        // Load snapshot
        let (restored_sim, _) = load_simulator_snapshot(&save_path).unwrap();

        // Verify index is rebuilt after restore
        assert_eq!(
            restored_sim
                .world
                .get_station_at_tile(&TileIndex::new(3, 4))
                .map(|s| s.id),
            Some(StationID(42))
        );
        assert_eq!(
            restored_sim
                .world
                .get_station_at_tile(&TileIndex::new(3, 5))
                .map(|s| s.id),
            Some(StationID(42))
        );
        assert_eq!(
            restored_sim
                .world
                .get_station_at_tile(&TileIndex::new(0, 0)),
            None
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validation_rejects_out_of_bounds_vehicle() {
        let mut sim = Simulator::new(MapSize::new(5, 5));
        let people = PeopleSimulator::new();

        let vehicle = Vehicle::new(
            VehicleID(1),
            CompanyID(1),
            transport_types::EngineID(1),
            transport_types::enum_::VehicleKind::Ship,
            TileIndex::new(10, 10), // out of 5x5 bounds
            OrderListID(1),
        );
        sim.world.vehicles.insert(VehicleID(1), vehicle);

        let envelope = SnapshotEnvelope::from_simulator(&sim, &people);
        let result = validate_snapshot(&envelope);
        assert!(matches!(result, Err(Error::Validation(_))));
    }
}
