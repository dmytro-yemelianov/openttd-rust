use transport_render::backend::{Color, Heading8, NullBackend, RenderBackend, ScreenRect, SpriteId, Vec2};
use transport_render::camera::{Camera, ProjectionMode};
use transport_render::depth::{
    calculate_isometric_depth, sort_render_items, RenderItem, RenderLayer,
};
use transport_render::interpolator::FrameInterpolator;
use transport_render::scene::SceneBuilder;
use transport_sim::World;
use transport_types::enum_::{TileKind, VehicleKind, VehicleState};
use transport_types::{
    BuildingID, CompanyID, EngineID, Money, OrderListID, StationID, TileIndex, TownID, VehicleID,
};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};
use transport_world::map::MapSize;
use transport_world::town::{Building, BuildingKind, Town};

#[test]
fn test_null_backend_recording() {
    let mut backend = NullBackend::new();
    backend.begin_frame(800, 600, Color::BLACK);
    backend.draw_rect(ScreenRect::new(10.0, 10.0, 100.0, 50.0), Color::RED, true);
    backend.draw_sprite(
        Vec2::new(150.0, 150.0),
        SpriteId::Terrain(TileKind::Grass),
        100.0,
        Color::WHITE,
    );
    backend.draw_text(Vec2::new(20.0, 20.0), "Hello Simulation", 16.0, Color::GREEN);
    backend.end_frame();

    assert_eq!(backend.frame_count, 1);
    assert_eq!(backend.sprite_count(), 1);
    assert_eq!(backend.commands.len(), 5);
}

#[test]
fn test_camera_coordinate_roundtrip() {
    let mut camera = Camera::new(1000.0, 800.0, ProjectionMode::Isometric);
    camera.offset = Vec2::new(50.0, 30.0);
    camera.set_zoom(1.0);

    let test_tiles = [
        TileIndex::new(5, 5),
        TileIndex::new(12, 8),
        TileIndex::new(20, 15),
        TileIndex::new(0, 0),
    ];

    for &tile in &test_tiles {
        let screen_pos = camera.world_to_screen(tile, (8, 8), 0);
        let recovered = camera.screen_to_world(screen_pos);
        assert_eq!(
            recovered,
            Some(tile),
            "Isometric world->screen->world roundtrip must recover original tile"
        );
    }

    // Orthographic projection roundtrip
    camera.projection = ProjectionMode::Orthographic;
    for &tile in &test_tiles {
        let screen_pos = camera.world_to_screen(tile, (8, 8), 0);
        let recovered = camera.screen_to_world(screen_pos);
        assert_eq!(
            recovered,
            Some(tile),
            "Orthographic world->screen->world roundtrip must recover original tile"
        );
    }
}

#[test]
fn test_camera_frustum_culling_bounds() {
    let camera = Camera::new(600.0, 400.0, ProjectionMode::Isometric);
    let (min_t, max_t) = camera.visible_tile_bounds(200, 200);

    // Visible box must be a small bounded sub-window, not the whole 200x200 map
    let visible_area = (max_t.x - min_t.x) as u32 * (max_t.y - min_t.y) as u32;
    assert!(
        visible_area < 200 * 200,
        "Frustum culling must strictly bound visible tiles to viewport size"
    );
    assert!(max_t.x < 200 && max_t.y < 200);
}

#[test]
fn test_sub_tick_frame_interpolation() {
    let mut interpolator = FrameInterpolator::new();
    let camera = Camera::new(800.0, 600.0, ProjectionMode::Isometric);
    let vid = VehicleID(42);

    let pos1 = TileIndex::new(4, 4);
    let pos2 = TileIndex::new(8, 8);

    interpolator.update_vehicle(vid, pos1, (8, 8));
    interpolator.update_vehicle(vid, pos2, (8, 8));

    // Alpha 0.0 -> at pos1
    let (screen_0, heading_0) = interpolator
        .interpolate_vehicle(vid, 0.0, &camera, 0)
        .unwrap();
    let expected_0 = camera.world_to_screen(pos1, (8, 8), 0);
    assert_eq!(screen_0, expected_0);
    assert_eq!(heading_0, Heading8::SouthEast);

    // Alpha 1.0 -> at pos2
    let (screen_1, _) = interpolator
        .interpolate_vehicle(vid, 1.0, &camera, 0)
        .unwrap();
    let expected_1 = camera.world_to_screen(pos2, (8, 8), 0);
    assert_eq!(screen_1, expected_1);

    // Alpha 0.5 -> exactly at midpoint
    let (screen_half, _) = interpolator
        .interpolate_vehicle(vid, 0.5, &camera, 0)
        .unwrap();
    let expected_half = Vec2::new(
        (expected_0.x + expected_1.x) / 2.0,
        (expected_0.y + expected_1.y) / 2.0,
    );
    assert!((screen_half.x - expected_half.x).abs() < 0.01);
    assert!((screen_half.y - expected_half.y).abs() < 0.01);
}

#[test]
fn test_isometric_depth_sorting_invariants() {
    let mut items = vec![
        RenderItem::new(
            Vec2::ZERO,
            SpriteId::Building(BuildingKind::Commercial),
            calculate_isometric_depth(10, 10, 0, RenderLayer::Building),
            Color::WHITE,
        ),
        RenderItem::new(
            Vec2::ZERO,
            SpriteId::Terrain(TileKind::Grass),
            calculate_isometric_depth(2, 2, 0, RenderLayer::Ground),
            Color::WHITE,
        ),
        RenderItem::new(
            Vec2::ZERO,
            SpriteId::Vehicle(VehicleKind::Ship, Heading8::North),
            calculate_isometric_depth(5, 5, 0, RenderLayer::Vehicle),
            Color::WHITE,
        ),
    ];

    sort_render_items(&mut items);

    // Background tile (2, 2) must be first, followed by (5, 5), followed by (10, 10)
    assert_eq!(items[0].sprite, SpriteId::Terrain(TileKind::Grass));
    assert!(matches!(items[1].sprite, SpriteId::Vehicle(..)));
    assert_eq!(
        items[2].sprite,
        SpriteId::Building(BuildingKind::Commercial)
    );
}

#[test]
fn test_end_to_end_scene_builder_render() {
    let mut world = World::new(MapSize::new(30, 30));
    let company_id = CompanyID(1);
    world.companies.insert(
        company_id,
        Company::new(company_id, "Visual Ferry".into(), Money(5000), 0xFF0000),
    );

    let dock_tile = TileIndex::new(5, 5);
    let st_id = StationID(10);
    world.stations.insert(
        st_id,
        Station::new(st_id, "South Pier".into(), vec![dock_tile]),
    );
    world.tile_to_station.insert(dock_tile, st_id);

    let town_id = TownID(1);
    let mut town = Town::new(town_id, "Pier City".into(), dock_tile);
    town.building_ids.push(BuildingID(1));
    world.towns.insert(town_id, town);
    world.buildings.insert(
        BuildingID(1),
        Building::new(
            BuildingID(1),
            town_id,
            TileIndex::new(6, 5),
            BuildingKind::Commercial,
            30,
        ),
    );

    let ship_id = VehicleID(1);
    let mut ship = Vehicle::new(
        ship_id,
        company_id,
        EngineID(1),
        VehicleKind::Ship,
        dock_tile,
        OrderListID(1),
    );
    ship.state = VehicleState::Traveling;
    world.vehicles.insert(ship_id, ship);

    let camera = Camera::new(800.0, 600.0, ProjectionMode::Isometric);
    let interpolator = FrameInterpolator::new();
    let mut builder = SceneBuilder::new();

    builder.build(&world, &interpolator, &camera, 0.5);

    assert!(
        !builder.items.is_empty(),
        "Scene graph must contain visible items"
    );

    let mut backend = NullBackend::new();
    builder.render(&mut backend, &camera, Color::BLACK);

    assert_eq!(backend.frame_count, 1);
    assert!(backend.sprite_count() >= builder.items.len());
}
