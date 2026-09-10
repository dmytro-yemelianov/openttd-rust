use serde::{Deserialize, Serialize};
use transport_types::{EngineID, StationID, TileIndex, TownID, VehicleID};

/// Active user interaction tool mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolMode {
    /// Inspect world entities: view tile kind, owner, station rating, town population
    Inspect,
    /// Construct a new passenger/cargo dock or station
    BuildStation {
        name: String,
        width: u16,
        height: u16,
    },
    /// Purchase and deploy a new vehicle at a selected tile
    BuildVehicle {
        engine_id: EngineID,
    },
    /// Demolish infrastructure or clear terrain
    Demolish,
    /// Interactive order scheduler: click stations to append GoToStation orders
    OrderRoute {
        vehicle_id: VehicleID,
    },
}

impl Default for ToolMode {
    fn default() -> Self {
        Self::Inspect
    }
}

/// Currently inspected or selected world entity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolSelection {
    Tile(TileIndex),
    Vehicle(VehicleID),
    Station(StationID),
    Town(TownID),
}

/// Interactive tool state machine managing cursor hovering, multi-tile drag boxes, and active modes.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolStateMachine {
    pub mode: ToolMode,
    pub hovered_tile: Option<TileIndex>,
    pub drag_start: Option<TileIndex>,
    pub drag_current: Option<TileIndex>,
    pub selection: Option<ToolSelection>,
}

impl ToolStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Change the active tool mode and clear pending drag states.
    pub fn set_mode(&mut self, mode: ToolMode) {
        self.mode = mode;
        self.cancel_drag();
    }

    /// Update the current cursor tile position under hover.
    pub fn on_hover(&mut self, tile: Option<TileIndex>) {
        self.hovered_tile = tile;
        if self.drag_start.is_some() {
            self.drag_current = tile;
        }
    }

    /// Begin a drag interaction (e.g. mouse button down on a tile).
    pub fn start_drag(&mut self, tile: TileIndex) {
        self.drag_start = Some(tile);
        self.drag_current = Some(tile);
    }

    /// Complete a drag interaction (e.g. mouse button released) and return the bounding corner tiles.
    pub fn end_drag(&mut self) -> Option<(TileIndex, TileIndex)> {
        let start = self.drag_start.take()?;
        let current = self.drag_current.take().unwrap_or(start);
        Some((start, current))
    }

    /// Cancel the active drag interaction without applying changes.
    pub fn cancel_drag(&mut self) {
        self.drag_start = None;
        self.drag_current = None;
    }

    /// Return the list of distinct tiles contained within the active drag bounding box.
    pub fn dragged_tiles(&self) -> Vec<TileIndex> {
        let (start, current) = match (self.drag_start, self.drag_current) {
            (Some(s), Some(c)) => (s, c),
            (Some(s), None) => (s, s),
            _ => {
                if let Some(h) = self.hovered_tile {
                    return vec![h];
                } else {
                    return Vec::new();
                }
            }
        };

        let min_x = start.x.min(current.x);
        let max_x = start.x.max(current.x);
        let min_y = start.y.min(current.y);
        let max_y = start.y.max(current.y);

        let mut tiles = Vec::with_capacity(((max_x - min_x + 1) as usize) * ((max_y - min_y + 1) as usize));
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                tiles.push(TileIndex::new(x, y));
            }
        }
        tiles
    }
}
