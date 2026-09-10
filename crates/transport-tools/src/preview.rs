use transport_render::backend::{Color, SpriteId};
use transport_render::camera::Camera;
use transport_render::depth::{calculate_isometric_depth, RenderItem, RenderLayer};
use transport_sim::World;
use transport_types::{CompanyID, StationID};

use crate::tool::{ToolMode, ToolStateMachine};
use crate::validator::{DryRunValidator, PlacementValidity};

/// Generates visual ghost preview items and catchment overlays for the active tool.
pub struct GhostOverlay;

impl GhostOverlay {
    /// Generate renderable ghost overlay items for the active tool state.
    pub fn generate_overlay_items(
        world: &World,
        tools: &ToolStateMachine,
        company_id: CompanyID,
        camera: &Camera,
    ) -> (Vec<RenderItem>, Option<PlacementValidity>) {
        let mut items = Vec::new();
        let mut validity_outcome = None;

        match &tools.mode {
            ToolMode::Inspect => {
                if let Some(tile) = tools.hovered_tile {
                    let elev = world.map.get(tile).map(|t| t.base.height).unwrap_or(0);
                    let screen_pos = camera.world_to_screen(tile, (8, 8), elev);
                    let depth = calculate_isometric_depth(tile.x, tile.y, elev, RenderLayer::Overlay);

                    items.push(RenderItem::new(
                        screen_pos,
                        SpriteId::Custom(0xFFEE), // Highlight box marker
                        depth,
                        Color::YELLOW,
                    ));
                }
            }
            ToolMode::BuildStation { .. } => {
                let candidate_tiles = tools.dragged_tiles();
                if candidate_tiles.is_empty() {
                    return (items, None);
                }

                let validity = DryRunValidator::validate_station_placement(
                    world,
                    company_id,
                    &candidate_tiles,
                );
                let is_valid = validity.is_valid();
                validity_outcome = Some(validity);

                let tint = if is_valid {
                    Color::rgba(50, 220, 50, 180) // Green
                } else {
                    Color::rgba(220, 50, 50, 180) // Red
                };

                for &t in &candidate_tiles {
                    let elev = world.map.get(t).map(|tile| tile.base.height).unwrap_or(0);
                    let screen_pos = camera.world_to_screen(t, (8, 8), elev);
                    let depth = calculate_isometric_depth(t.x, t.y, elev, RenderLayer::Overlay);

                    items.push(RenderItem::new(
                        screen_pos,
                        SpriteId::Station(StationID(0)), // Ghost station marker
                        depth,
                        tint,
                    ));
                }

                // If hovering/dragging, also show catchment area overlay in light blue
                if is_valid {
                    let catchment = DryRunValidator::calculate_catchment(world, &candidate_tiles);
                    for c_tile in catchment.catchment_tiles {
                        if !candidate_tiles.contains(&c_tile) {
                            let elev = world.map.get(c_tile).map(|tile| tile.base.height).unwrap_or(0);
                            let screen_pos = camera.world_to_screen(c_tile, (8, 8), elev);
                            let depth = calculate_isometric_depth(c_tile.x, c_tile.y, elev, RenderLayer::Overlay);

                            items.push(RenderItem::new(
                                screen_pos,
                                SpriteId::Custom(0xCA7C), // Catchment marker
                                depth,
                                Color::rgba(100, 180, 255, 100),
                            ));
                        }
                    }
                }
            }
            ToolMode::BuildVehicle { engine_id } => {
                if let Some(tile) = tools.hovered_tile {
                    let validity = DryRunValidator::validate_vehicle_placement(
                        world,
                        company_id,
                        *engine_id,
                        tile,
                    );
                    let tint = if validity.is_valid() {
                        Color::rgba(50, 220, 50, 180)
                    } else {
                        Color::rgba(220, 50, 50, 180)
                    };
                    validity_outcome = Some(validity);

                    let elev = world.map.get(tile).map(|t| t.base.height).unwrap_or(0);
                    let screen_pos = camera.world_to_screen(tile, (8, 8), elev);
                    let depth = calculate_isometric_depth(tile.x, tile.y, elev, RenderLayer::Overlay);

                    items.push(RenderItem::new(
                        screen_pos,
                        SpriteId::Vehicle(
                            transport_types::enum_::VehicleKind::Ship,
                            transport_render::backend::Heading8::North,
                        ),
                        depth,
                        tint,
                    ));
                }
            }
            _ => {}
        }

        (items, validity_outcome)
    }
}
