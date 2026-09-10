use transport_render::backend::Vec2;
use transport_render::camera::Camera;
use transport_sim::{Command, World};
use transport_types::enum_::{OrderType, VehicleKind};
use transport_types::{CapabilityToken, CompanyID, OrderListID, StationID};

use crate::tool::{ToolMode, ToolSelection, ToolStateMachine};
use crate::validator::DryRunValidator;

/// Agnostic input bridge converting screen pointer and keyboard events into simulation commands.
pub struct InputController {
    pub company_id: CompanyID,
    pub token: CapabilityToken,
    pub tools: ToolStateMachine,
    pub next_sequence: u32,
}

impl InputController {
    pub fn new(company_id: CompanyID, token: CapabilityToken) -> Self {
        Self {
            company_id,
            token,
            tools: ToolStateMachine::new(),
            next_sequence: 1,
        }
    }

    /// Handle pointer motion: unprojects screen coordinate to world TileIndex and updates hover/drag.
    pub fn handle_mouse_move(&mut self, screen_pos: Vec2, camera: &Camera) {
        let tile = camera.screen_to_world(screen_pos);
        self.tools.on_hover(tile);
    }

    /// Handle pointer button press: initiates drag rectangle at the clicked tile.
    pub fn handle_mouse_down(&mut self, screen_pos: Vec2, camera: &Camera) {
        if let Some(tile) = camera.screen_to_world(screen_pos) {
            self.tools.start_drag(tile);
        }
    }

    /// Handle pointer button release: finalizes tool action, performs dry-run validation,
    /// and constructs an authoritative Command if the placement is valid.
    pub fn handle_mouse_up(
        &mut self,
        screen_pos: Vec2,
        camera: &Camera,
        world: &World,
    ) -> Option<Command> {
        let released_tile = camera.screen_to_world(screen_pos);
        self.tools.on_hover(released_tile);

        let mode = self.tools.mode.clone();
        match mode {
            ToolMode::Inspect => {
                self.tools.cancel_drag();
                if let Some(tile) = released_tile {
                    if let Some(&sid) = world.tile_to_station.get(&tile) {
                        self.tools.selection = Some(ToolSelection::Station(sid));
                    } else if let Some((&vid, _)) = world.vehicles.iter().find(|(_, v)| v.position == tile) {
                        self.tools.selection = Some(ToolSelection::Vehicle(vid));
                    } else {
                        self.tools.selection = Some(ToolSelection::Tile(tile));
                    }
                }
                None
            }
            ToolMode::BuildStation { name, .. } => {
                let candidate_tiles = self.tools.dragged_tiles();
                self.tools.cancel_drag();

                if candidate_tiles.is_empty() {
                    return None;
                }

                let validity = DryRunValidator::validate_station_placement(
                    world,
                    self.company_id,
                    &candidate_tiles,
                );

                if validity.is_valid() {
                    Some(Command::BuildStation {
                        company_id: self.company_id,
                        name: name.clone(),
                        tiles: candidate_tiles,
                    })
                } else {
                    None
                }
            }
            ToolMode::BuildVehicle { engine_id } => {
                self.tools.cancel_drag();
                let target_tile = released_tile?;

                let validity = DryRunValidator::validate_vehicle_placement(
                    world,
                    self.company_id,
                    engine_id,
                    target_tile,
                );

                if validity.is_valid() {
                    let order_list_id = OrderListID(self.next_sequence);
                    self.next_sequence += 1;

                    Some(Command::PurchaseVehicle {
                        company_id: self.company_id,
                        engine_id,
                        kind: VehicleKind::Ship,
                        position: target_tile,
                        order_list_id,
                    })
                } else {
                    None
                }
            }
            ToolMode::OrderRoute { vehicle_id } => {
                self.tools.cancel_drag();
                let clicked_tile = released_tile?;
                let target_station_id: StationID = *world.tile_to_station.get(&clicked_tile)?;

                let vehicle = world.vehicles.get(&vehicle_id)?;
                let order = OrderType::GoToStation {
                    station_id: target_station_id,
                    conditions: None,
                };

                Some(Command::AddOrderToList {
                    order_list_id: vehicle.orders,
                    order,
                })
            }
            _ => {
                self.tools.cancel_drag();
                None
            }
        }
    }

    /// Cancel any pending interactions.
    pub fn cancel(&mut self) {
        self.tools.cancel_drag();
    }
}
