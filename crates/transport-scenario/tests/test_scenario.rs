use pretty_assertions::assert_eq;
use transport_scenario::{
    MapGenConfig, MapGenerator, ObjectiveTarget, ScenarioController, ScenarioDefinition,
    ScenarioStatus, TopologyAnalyzer,
};
use transport_types::unit::Money;
use transport_types::{CargoType, Ticks, TileIndex};
use transport_world::MapSize;

#[test]
fn test_deterministic_map_generation_reproducibility() {
    let config = MapGenConfig {
        size: MapSize::new(32, 32),
        seed: 12345,
        sea_level: 3,
        num_towns: 2,
        starting_capital: Money(50_000),
    };

    let world1 = MapGenerator::generate(&config);
    let world2 = MapGenerator::generate(&config);

    // Verify tile-for-tile identity across runs
    for (tile_idx, tile1) in world1.map.iter_enumerated() {
        let tile2 = world2.map.get(tile_idx).expect("Tile must exist in world2");
        assert_eq!(tile1.base.kind, tile2.base.kind);
        assert_eq!(tile1.base.height, tile2.base.height);
        assert_eq!(tile1.base.water_height, tile2.base.water_height);
    }

    // Verify town count and centers
    assert_eq!(world1.towns.len(), world2.towns.len());
    for (t_id, town1) in &world1.towns {
        let town2 = &world2.towns[t_id];
        assert_eq!(town1.center, town2.center);
        assert_eq!(town1.census_population, town2.census_population);
    }
}

#[test]
fn test_different_seeds_produce_different_terrain() {
    let config1 = MapGenConfig {
        size: MapSize::new(32, 32),
        seed: 11111,
        sea_level: 3,
        num_towns: 2,
        starting_capital: Money(50_000),
    };
    let config2 = MapGenConfig {
        size: MapSize::new(32, 32),
        seed: 99999,
        sea_level: 3,
        num_towns: 2,
        starting_capital: Money(50_000),
    };

    let world1 = MapGenerator::generate(&config1);
    let world2 = MapGenerator::generate(&config2);

    let differences = world1
        .map
        .iter_enumerated()
        .filter(|(idx, t1)| {
            let t2 = world2.map.get(*idx).unwrap();
            t1.base.kind != t2.base.kind || t1.base.height != t2.base.height
        })
        .count();

    assert!(
        differences > 100,
        "Different seeds must generate distinct terrain"
    );
}

#[test]
fn test_topology_water_body_reachability() {
    let config = MapGenConfig {
        size: MapSize::new(48, 48),
        seed: 42,
        sea_level: 4,
        num_towns: 3,
        starting_capital: Money(50_000),
    };

    let world = MapGenerator::generate(&config);
    let topology = TopologyAnalyzer::analyze(&world.map, 20);

    // Find any water tile
    let water_tiles: Vec<TileIndex> = world
        .map
        .iter_enumerated()
        .filter(|(_, t)| t.is_water())
        .map(|(idx, _)| idx)
        .collect();

    assert!(!water_tiles.is_empty(), "Map must have generated water tiles");

    let first_water = water_tiles[0];
    let body_id = topology
        .water_body_at(first_water)
        .expect("Water tile must have a water body ID");

    // Adjacent water tiles in the same component must be connected
    let coastal_candidates = topology.find_coastal_land_tiles(&world.map, body_id);
    assert!(
        !coastal_candidates.is_empty(),
        "Navigable body must have adjacent coastal dry land"
    );

    for coastal in coastal_candidates {
        let tile = world.map.get(coastal).unwrap();
        assert!(!tile.is_water(), "Coastal candidate must be dry land");
    }
}

#[test]
fn test_scenario_controller_victory_lifecycle() {
    let config = MapGenConfig {
        size: MapSize::new(32, 32),
        seed: 42,
        sea_level: 3,
        num_towns: 2,
        starting_capital: Money(100_000),
    };
    let mut world = MapGenerator::generate(&config);

    let definition = ScenarioDefinition {
        id: "test_scenario".to_string(),
        title: "Test Port".to_string(),
        briefing: "Deliver 50 cargo units".to_string(),
        map_config: config,
        objectives: vec![
            ObjectiveTarget::CargoDelivered {
                cargo_type: CargoType(1),
                target_amount: 50,
            },
            ObjectiveTarget::PassengerTrips { target_trips: 20 },
        ],
        time_limit_ticks: Some(1_000),
    };

    let mut controller = ScenarioController::new(definition);

    // Initial state: InProgress
    let status = controller.update(&world);
    assert!(matches!(status, ScenarioStatus::InProgress { .. }));

    // Advance partial progress
    controller.record_cargo_delivered(CargoType(1), 30);
    controller.record_passenger_trips(10);
    world.tick = Ticks(50);
    let status = controller.update(&world);
    assert!(matches!(status, ScenarioStatus::InProgress { .. }));

    // Complete objectives
    controller.record_cargo_delivered(CargoType(1), 25); // total 55 >= 50
    controller.record_passenger_trips(15); // total 25 >= 20
    world.tick = Ticks(120);

    let status = controller.update(&world);
    match status {
        ScenarioStatus::Victory { completed_at_tick } => {
            assert_eq!(*completed_at_tick, 120);
        }
        other => panic!("Expected Victory, got {other:?}"),
    }
}

#[test]
fn test_scenario_controller_defeat_on_timeout() {
    let config = MapGenConfig {
        size: MapSize::new(32, 32),
        seed: 42,
        sea_level: 3,
        num_towns: 2,
        starting_capital: Money(100_000),
    };
    let mut world = MapGenerator::generate(&config);

    let definition = ScenarioDefinition {
        id: "timeout_scenario".to_string(),
        title: "Timeout Test".to_string(),
        briefing: "Impossible target within 50 ticks".to_string(),
        map_config: config,
        objectives: vec![ObjectiveTarget::PassengerTrips { target_trips: 1000 }],
        time_limit_ticks: Some(50),
    };

    let mut controller = ScenarioController::new(definition);

    // Tick 10: InProgress
    world.tick = Ticks(10);
    assert!(matches!(
        controller.update(&world),
        ScenarioStatus::InProgress { .. }
    ));

    // Tick 50: Exceeds time limit -> Defeat
    world.tick = Ticks(50);
    match controller.update(&world) {
        ScenarioStatus::Defeat { tick, reason } => {
            assert_eq!(*tick, 50);
            assert!(reason.contains("Time limit"));
        }
        other => panic!("Expected Defeat, got {other:?}"),
    }
}
