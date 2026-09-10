use std::fs;
use transport_sim::drivers::{LogisticsDriver, MovementDriver};
use transport_sim::kernel::Microkernel;
use transport_sim::oracle::{project_state_slice, DivergenceDetector, OracleTraceFrame};
use transport_types::enum_::{OrderType, VehicleKind};
use transport_types::{
    CargoAmount, CargoType, CompanyID, EngineID, Money, OrderListID, StationID, TileIndex,
    VehicleID,
};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};
use transport_world::map::MapSize;

fn setup_standard_ship_voyage() -> (Microkernel, CompanyID, VehicleID, StationID, StationID) {
    let mut kernel = Microkernel::new(MapSize::new(20, 20));
    let company_id = CompanyID(1);
    kernel.world.companies.insert(
        company_id,
        Company::new(company_id, "Maritime Cargo".into(), Money(10000), 0x00FF00),
    );

    let dock_a = StationID(1);
    let dock_b = StationID(2);
    let tile_a = TileIndex::new(2, 2);
    let tile_b = TileIndex::new(6, 2);

    let mut st_a = Station::new(dock_a, "Port Origin".into(), vec![tile_a]);
    st_a.company_id = Some(company_id);
    let mut st_b = Station::new(dock_b, "Port Terminus".into(), vec![tile_b]);
    st_b.company_id = Some(company_id);

    kernel.world.stations.insert(dock_a, st_a);
    kernel.world.stations.insert(dock_b, st_b);
    kernel.world.map.set_station_at(tile_a, dock_a, Some(company_id)).unwrap();
    kernel.world.map.set_station_at(tile_b, dock_b, Some(company_id)).unwrap();
    kernel.world.tile_to_station.insert(tile_a, dock_a);
    kernel.world.tile_to_station.insert(tile_b, dock_b);

    let orders_id = OrderListID(1);
    kernel.world.order_lists.insert(
        orders_id,
        vec![OrderType::GoToStation {
            station_id: dock_b,
            conditions: None,
        }],
    );

    let ship_id = VehicleID(1);
    let mut ship = Vehicle::new(
        ship_id,
        company_id,
        EngineID(1),
        VehicleKind::Ship,
        tile_a,
        orders_id,
    );
    ship.cargo.push((CargoType(1), CargoAmount(40))); // 40 units of Grain
    kernel.world.vehicles.insert(ship_id, ship);

    (kernel, company_id, ship_id, dock_a, dock_b)
}

#[test]
fn test_oracle_parity_with_golden_reference() {
    let (mut kernel, _, _ship_id, _, _) = setup_standard_ship_voyage();

    kernel.register_driver(MovementDriver::new());
    kernel.register_driver(LogisticsDriver::new());

    let mut recorded_frames = Vec::new();
    // Record tick 0 (initial state)
    recorded_frames.push(project_state_slice(&kernel.world));

    // Execute 5 ticks: traveling across tiles (3,2), (4,2), (5,2) and arriving at dock (6,2)
    for _ in 1..=5 {
        kernel.tick().unwrap();
        recorded_frames.push(project_state_slice(&kernel.world));
    }

    // Load golden reference trace fixture
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/oracle_ship_voyage.json"
    );
    let fixture_json = fs::read_to_string(fixture_path).expect("Fixture file must exist");
    let golden_frames: Vec<OracleTraceFrame> =
        serde_json::from_str(&fixture_json).expect("Fixture must be valid JSON");

    // Perform differential comparison
    let report = DivergenceDetector::compare(&recorded_frames, &golden_frames);

    assert!(
        report.is_equivalent(),
        "Simulation trace diverged from Oracle golden reference! Divergence: {:?}",
        report.first_divergence
    );
    assert_eq!(report.matched_ticks, 6);
}

#[test]
fn test_divergence_detector_pinpoints_exact_first_mutation() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/oracle_ship_voyage.json"
    );
    let fixture_json = fs::read_to_string(fixture_path).unwrap();
    let golden_frames: Vec<OracleTraceFrame> = serde_json::from_str(&fixture_json).unwrap();

    let mut mutated_trace = golden_frames.clone();

    // Introduce synthetic divergence at Tick 3: alter vehicle position
    let veh_trace = mutated_trace[3]
        .vehicles
        .get_mut(&VehicleID(1))
        .expect("Vehicle 1 must exist at tick 3");
    veh_trace.position = TileIndex::new(99, 99);

    let report = DivergenceDetector::compare(&mutated_trace, &golden_frames);

    assert!(!report.is_equivalent());
    assert_eq!(report.matched_ticks, 3); // Ticks 0, 1, 2 matched

    let div = report.first_divergence.unwrap();
    assert_eq!(div.tick, 3);
    assert_eq!(div.entity, "Vehicle(VehicleID(1))");
    assert_eq!(div.field, "position");
    assert_eq!(div.rust_value, format!("{:?}", TileIndex::new(99, 99)));
    assert_eq!(div.oracle_value, format!("{:?}", TileIndex::new(5, 2)));
}

#[test]
fn test_oracle_projection_ignores_non_compatible_peep_extensions() {
    let (mut kernel, _, _, _, _) = setup_standard_ship_voyage();

    kernel.register_driver(MovementDriver::new());
    kernel.register_driver(LogisticsDriver::new());

    let mut recorded_frames = Vec::new();
    recorded_frames.push(project_state_slice(&kernel.world));

    for _ in 1..=5 {
        kernel.tick().unwrap();
        recorded_frames.push(project_state_slice(&kernel.world));
    }

    // Load golden reference
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/oracle_ship_voyage.json"
    );
    let fixture_json = fs::read_to_string(fixture_path).unwrap();
    let golden_frames: Vec<OracleTraceFrame> = serde_json::from_str(&fixture_json).unwrap();

    let report = DivergenceDetector::compare(&recorded_frames, &golden_frames);
    assert!(report.is_equivalent());
}
