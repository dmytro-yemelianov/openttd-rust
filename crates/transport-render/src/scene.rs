use transport_sim::World;
use transport_types::TileIndex;

use crate::backend::{Color, RenderBackend, SpriteId};
use crate::camera::Camera;
use crate::depth::{calculate_isometric_depth, sort_render_items, RenderItem, RenderLayer};
use crate::interpolator::FrameInterpolator;

/// Scene graph builder that culls, depth-sorts, and dispatches renderable items.
#[derive(Debug, Default, Clone)]
pub struct SceneBuilder {
    pub items: Vec<RenderItem>,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a culled and depth-sorted scene graph from current world state and sub-tick interpolator.
    pub fn build(
        &mut self,
        world: &World,
        interpolator: &FrameInterpolator,
        camera: &Camera,
        alpha: f32,
    ) {
        self.items.clear();

        let map_size = world.map.size();
        let (min_tile, max_tile) = camera.visible_tile_bounds(map_size.width, map_size.height);

        // 1. Cull and collect visible terrain tiles and stations
        for y in min_tile.y..=max_tile.y {
            for x in min_tile.x..=max_tile.x {
                let tile_idx = TileIndex::new(x, y);
                let tile = match world.map.get(tile_idx) {
                    Some(t) => t,
                    None => continue,
                };

                let screen_pos = camera.world_to_screen(tile_idx, (8, 8), tile.base.height);
                let depth = calculate_isometric_depth(x, y, tile.base.height, RenderLayer::Ground);

                self.items.push(RenderItem::new(
                    screen_pos,
                    SpriteId::Terrain(tile.base.kind),
                    depth,
                    Color::WHITE,
                ));

                // Station overlay if this tile belongs to a station
                if let Some(&station_id) = world.tile_to_station.get(&tile_idx) {
                    let st_depth =
                        calculate_isometric_depth(x, y, tile.base.height, RenderLayer::Station);
                    self.items.push(RenderItem::new(
                        screen_pos,
                        SpriteId::Station(station_id),
                        st_depth,
                        Color::WHITE,
                    ));
                }
            }
        }

        // 2. Visible buildings
        for bldg in world.buildings.values() {
            if bldg.tile.x >= min_tile.x
                && bldg.tile.x <= max_tile.x
                && bldg.tile.y >= min_tile.y
                && bldg.tile.y <= max_tile.y
            {
                let elev = world
                    .map
                    .get(bldg.tile)
                    .map(|t| t.base.height)
                    .unwrap_or(0);
                let screen_pos = camera.world_to_screen(bldg.tile, (8, 8), elev);
                let depth =
                    calculate_isometric_depth(bldg.tile.x, bldg.tile.y, elev, RenderLayer::Building);

                self.items.push(RenderItem::new(
                    screen_pos,
                    SpriteId::Building(bldg.kind),
                    depth,
                    Color::WHITE,
                ));
            }
        }

        // 3. Visible vehicles with sub-tick interpolated positions
        for (&vid, vehicle) in &world.vehicles {
            let elev = world
                .map
                .get(vehicle.position)
                .map(|t| t.base.height)
                .unwrap_or(0);

            let (screen_pos, heading) = interpolator
                .interpolate_vehicle(vid, alpha, camera, elev)
                .unwrap_or_else(|| {
                    let pos = camera.world_to_screen(vehicle.position, (8, 8), elev);
                    (pos, crate::backend::Heading8::North)
                });

            let depth = calculate_isometric_depth(
                vehicle.position.x,
                vehicle.position.y,
                elev,
                RenderLayer::Vehicle,
            );

            self.items.push(RenderItem::new(
                screen_pos,
                SpriteId::Vehicle(vehicle.kind, heading),
                depth,
                Color::WHITE,
            ));
        }

        // 4. Stable sort all items back-to-front by depth (Painter's algorithm)
        sort_render_items(&mut self.items);
    }

    /// Dispatch the sorted scene graph to a pluggable render backend.
    pub fn render(&self, backend: &mut dyn RenderBackend, camera: &Camera, clear_color: Color) {
        backend.begin_frame(
            camera.viewport_width as u32,
            camera.viewport_height as u32,
            clear_color,
        );

        for item in &self.items {
            backend.draw_sprite(item.pos, item.sprite, item.depth, item.tint);
        }

        backend.end_frame();
    }
}
