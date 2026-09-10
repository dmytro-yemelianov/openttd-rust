use transport_sim::drivers::MovementDriver;
use transport_sim::kernel::Microkernel;
use transport_sim::solvers::water_path::{PassabilityMode, PathResult, WaterPathSolver};
use transport_types::enum_::{OrderType, TileKind, VehicleKind, VehicleState};
use transport_types::{
    CompanyID, EngineID, Money, OrderListID, StationID, TileIndex, VehicleID,
};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};
use transport_world::map::{Map, MapSize, Tile};

#[test]
fn test_pathfinder_circumnavigates_land_obstacle() {
    let size = MapSize::new(10, 10);
    let mut map = Map::new(size);

    // Fill map with water
    for y in 0..10 {
        for x in 0..10 {
            let mut t = Tile::new_empty();
            t.base.kind = TileKind::Water;
            map.set(TileIndex::new(x, y), t).unwrap();
        }
    }

    // Place a vertical land barrier at x = 4, covering y = 1..=4
    for y in 1..=4 {
        let mut t = Tile::new_empty();
        t.base.kind = TileKind::Clear; // Land obstacle
        map.set(TileIndex::new(4, y), t).unwrap();
    }

    let start = TileIndex::new(2, 2);
    let target = TileIndex::new(6, 2);

    let mut solver = WaterPathSolver::new().with_passability_mode(PassabilityMode::StrictWater);
    let result = solver.find_path(start, target, &map, 256);

    match result {
        PathResult::Found(path) => {
            assert!(!path.is_empty(), "Path should contain steps");
            assert_eq!(*path.last().unwrap(), target, "Path must end at target");

            // Ensure no step touches the land barrier (x=4, y=1..=4)
            for step in &path {
                let is_in_barrier = step.x == 4 && (1..=4).contains(&step.y);
                assert!(!is_in_barrier, "Path step {step:?} passed through land barrier!");
            }
        }
        other => panic!("Expected path to be found, got {other:?}"),
    }
}

#[test]
fn test_pathfinder_respects_expansion_budget() {
    let size = MapSize::new(20, 20);
    let mut map = Map::new(size);

    for y in 0..20 {
        for x in 0..20 {
            let mut t = Tile::new_empty();
            t.base.kind = TileKind::Water;
            map.set(TileIndex::new(x, y), t).unwrap();
        }
    }

    let start = TileIndex::new(0, 0);
    let target = TileIndex::new(19, 19);

    let mut solver = WaterPathSolver::new();
    // Budget of only 5 expansions for a distance of 38 tiles
    let result = solver.find_path(start, target, &map, 5);

    assert_eq!(
        result,
        PathResult::BudgetExceeded,
        "Search must abort with BudgetExceeded when budget is exhausted"
    );
}

#[test]
fn test_pathfinder_identifies_unreachable_target() {
    let size = MapSize::new(10, 10);
    let mut map = Map::new(size);

    for y in 0..10 {
        for x in 0..10 {
            let mut t = Tile::new_empty();
            t.base.kind = TileKind::Water;
            map.set(TileIndex::new(x, y), t).unwrap();
        }
    }

    let target = TileIndex::new(5, 5);

    // Enclose target with an impassable ring of land
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let wall = TileIndex::new((5 + dx) as u16, (5 + dy) as u16);
            let mut t = Tile::new_empty();
            t.base.kind = TileKind::Clear; // Land
            map.set(wall, t).unwrap();
        }
    }

    let mut solver = WaterPathSolver::new();
    let start = TileIndex::new(0, 0);
    let result = solver.find_path(start, target, &map, 500);

    assert_eq!(
        result,
        PathResult::Unreachable,
        "Target enclosed in land must be reported as Unreachable"
    );
}

#[test]
fn test_microkernel_solver_phase_drives_ship_around_peninsula() {
    let mut kernel = Microkernel::new(MapSize::new(10, 10));

    // Fill map with water
    for y in 0..10 {
        for x in 0..10 {
            let mut t = Tile::new_empty();
            t.base.kind = TileKind::Water;
            kernel.world.map.set(TileIndex::new(x, y), t).unwrap();
        }
    }

    // Peninsula barrier from y=0 to y=3 at x=4
    for y in 0..=3 {
        let mut t = Tile::new_empty();
        t.base.kind = TileKind::Clear; // Land
        kernel.world.map.set(TileIndex::new(4, y), t).unwrap();
    }

    let company_id = CompanyID(1);
    kernel.world.companies.insert(
        company_id,
        Company::new(company_id, "Oceanic Line".into(), Money(1000), 0x112233),
    );

    let dock_a = StationID(1);
    let dock_b = StationID(2);
    let tile_a = TileIndex::new(2, 2);
    let tile_b = TileIndex::new(6, 2);

    let mut st_a = Station::new(dock_a, "West Dock".into(), vec![tile_a]);
    st_a.company_id = Some(company_id);
    let mut st_b = Station::new(dock_b, "East Dock".into(), vec![tile_b]);
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
    ship.state = VehicleState::Traveling;
    kernel.world.vehicles.insert(ship_id, ship);

    // Register WaterPathSolver (Phase::Solvers) and MovementDriver (Phase::Drivers)
    kernel.register_driver(WaterPathSolver::new());
    kernel.register_driver(MovementDriver::new());

    // Advance simulation until ship reaches East Dock
    let mut reached = false;
    for _ in 0..25 {
        kernel.tick().unwrap();
        let pos = kernel.world.vehicles.get(&ship_id).unwrap().position;

        // Verify the ship NEVER entered the peninsula barrier (x=4, y=0..=3)
        assert!(
            !(pos.x == 4 && pos.y <= 3),
            "Ship crashed into land barrier at {pos:?}"
        );

        if pos == tile_b {
            reached = true;
            break;
        }
    }

    assert!(reached, "Ship must reach East Dock by circumnavigating peninsula!");
}
