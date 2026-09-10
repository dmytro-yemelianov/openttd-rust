use transport_render::camera::{Camera, ProjectionMode};
use transport_sim::{Command, World};
use transport_tools::input::InputController;
use transport_tools::preview::GhostOverlay;
use transport_tools::tool::{ToolMode, ToolSelection, ToolStateMachine};
use transport_tools::validator::{DryRunValidator, PlacementValidity};
use transport_types::{
    BuildingID, CapabilityToken, CompanyID, Money, StationID, TileIndex, TownID,
};
use transport_world::definitions::Station;
use transport_world::entities::Company;
use transport_world::map::MapSize;
use transport_world::town::{Building, BuildingKind, Town};

#[test]
fn test_tool_state_machine_drag_and_hover() {
    let mut state = ToolStateMachine::new();
    assert_eq!(state.mode, ToolMode::Inspect);

    let t1 = TileIndex::new(2, 2);
    let t2 = TileIndex::new(4, 5);

    state.on_hover(Some(t1));
    assert_eq!(state.hovered_tile, Some(t1));

    state.start_drag(t1);
    state.on_hover(Some(t2));

    let dragged = state.dragged_tiles();
    // 3 x 4 = 12 tiles in drag rectangle
    assert_eq!(dragged.len(), 12);
    assert!(dragged.contains(&t1));
    assert!(dragged.contains(&t2));

    state.cancel_drag();
    assert_eq!(state.drag_start, None);
    assert_eq!(state.drag_current, None);
}

#[test]
fn test_dry_run_station_placement_and_cost() {
    let mut world = World::new(MapSize::new(20, 20));
    let company_id = CompanyID(1);
    world.companies.insert(
        company_id,
        Company::new(company_id, "Test Rail".into(), Money(5000), 0x00FF00),
    );

    let tiles = vec![TileIndex::new(5, 5), TileIndex::new(5, 6)];

    // 1. Valid placement
    let val = DryRunValidator::validate_station_placement(&world, company_id, &tiles);
    assert_eq!(val, PlacementValidity::Valid { cost: Money(2000) });

    // 2. Insufficient funds
    world.companies.get_mut(&company_id).unwrap().money = Money(500);
    let val_broke = DryRunValidator::validate_station_placement(&world, company_id, &tiles);
    assert!(matches!(val_broke, PlacementValidity::InsufficientFunds { .. }));

    // 3. Tile occupied
    world.tile_to_station.insert(TileIndex::new(5, 5), StationID(99));
    let val_occupied = DryRunValidator::validate_station_placement(&world, company_id, &tiles);
    assert_eq!(val_occupied, PlacementValidity::TileOccupied(TileIndex::new(5, 5)));

    // 4. Out of bounds
    let val_oob = DryRunValidator::validate_station_placement(
        &world,
        company_id,
        &[TileIndex::new(50, 50)],
    );
    assert_eq!(val_oob, PlacementValidity::OutOfBounds(TileIndex::new(50, 50)));
}

#[test]
fn test_catchment_area_calculation() {
    let mut world = World::new(MapSize::new(30, 30));
    let town_id = TownID(1);
    let mut town = Town::new(town_id, "CatchTown".into(), TileIndex::new(10, 10));
    town.building_ids.push(BuildingID(1));
    town.building_ids.push(BuildingID(2));
    world.towns.insert(town_id, town);

    // Nearby building at (12, 11): dx = 2, dy = 1 -> dist = 3 <= 3
    world.buildings.insert(
        BuildingID(1),
        Building::new(
            BuildingID(1),
            town_id,
            TileIndex::new(12, 11),
            BuildingKind::Residential,
            40,
        ),
    );

    // Far building at (20, 20): dist = 20 > 3
    world.buildings.insert(
        BuildingID(2),
        Building::new(
            BuildingID(2),
            town_id,
            TileIndex::new(20, 20),
            BuildingKind::Commercial,
            25,
        ),
    );

    let station_tiles = [TileIndex::new(10, 10)];
    let catchment = DryRunValidator::calculate_catchment(&world, &station_tiles);

    assert!(catchment.catchment_tiles.contains(&TileIndex::new(12, 11)));
    assert!(!catchment.catchment_tiles.contains(&TileIndex::new(20, 20)));
    assert_eq!(catchment.residential_capacity, 40);
    assert_eq!(catchment.workplace_capacity, 0);
    assert_eq!(catchment.towns_served, vec![town_id]);
}

#[test]
fn test_ghost_overlay_render_items() {
    let mut world = World::new(MapSize::new(20, 20));
    let company_id = CompanyID(1);
    world.companies.insert(
        company_id,
        Company::new(company_id, "Ghost Transit".into(), Money(10000), 0xFF00FF),
    );

    let mut state = ToolStateMachine::new();
    state.set_mode(ToolMode::BuildStation {
        name: "North Dock".into(),
        width: 1,
        height: 1,
    });
    state.start_drag(TileIndex::new(5, 5));

    let camera = Camera::new(800.0, 600.0, ProjectionMode::Isometric);
    let (items, validity) = GhostOverlay::generate_overlay_items(&world, &state, company_id, &camera);

    assert!(validity.unwrap().is_valid());
    // Must contain ghost station tile + catchment area highlight tiles
    assert!(!items.is_empty());
}

#[test]
fn test_input_controller_command_generation() {
    let mut world = World::new(MapSize::new(20, 20));
    let company_id = CompanyID(1);
    world.companies.insert(
        company_id,
        Company::new(company_id, "Click Corp".into(), Money(10000), 0x00FFFF),
    );

    let camera = Camera::new(800.0, 600.0, ProjectionMode::Orthographic);
    let mut controller = InputController::new(company_id, CapabilityToken::Company(company_id));

    // 1. BuildStation interaction
    controller.tools.set_mode(ToolMode::BuildStation {
        name: "Click Dock".into(),
        width: 1,
        height: 1,
    });

    let screen_pos = camera.world_to_screen(TileIndex::new(4, 4), (0, 0), 0);
    controller.handle_mouse_down(screen_pos, &camera);
    let cmd = controller.handle_mouse_up(screen_pos, &camera, &world);

    assert!(matches!(
        cmd,
        Some(Command::BuildStation {
            tiles,
            ..
        }) if tiles == vec![TileIndex::new(4, 4)]
    ));

    // 2. Inspect mode interaction on an existing station
    let st_id = StationID(5);
    world.tile_to_station.insert(TileIndex::new(7, 7), st_id);
    world.stations.insert(
        st_id,
        Station::new(st_id, "Inspect Pier".into(), vec![TileIndex::new(7, 7)]),
    );

    controller.tools.set_mode(ToolMode::Inspect);
    let inspect_screen = camera.world_to_screen(TileIndex::new(7, 7), (0, 0), 0);
    controller.handle_mouse_up(inspect_screen, &camera, &world);

    assert_eq!(
        controller.tools.selection,
        Some(ToolSelection::Station(st_id))
    );
}
