use serde::{Deserialize, Serialize};

use crate::backend::{Color, SpriteId, Vec2};

/// Visual rendering layer used to compute visual depth offsets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RenderLayer {
    /// Ground terrain tile
    Ground = 0,
    /// Water surface
    Water = 1,
    /// Infrastructure: docks, track, roads
    Infrastructure = 2,
    /// Stations and platforms
    Station = 3,
    /// Moving vehicles (ships, trains, road vehicles)
    Vehicle = 4,
    /// Peeps and pedestrians
    Pedestrian = 5,
    /// Physical buildings and factories
    Building = 6,
    /// UI cursor, ghost building preview, overlays
    Overlay = 7,
}

impl RenderLayer {
    pub fn offset(self) -> f32 {
        self as u8 as f32
    }
}

/// A depth-sorted renderable visual element in the scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderItem {
    pub pos: Vec2,
    pub sprite: SpriteId,
    pub depth: f32,
    pub tint: Color,
}

impl RenderItem {
    pub fn new(pos: Vec2, sprite: SpriteId, depth: f32, tint: Color) -> Self {
        Self {
            pos,
            sprite,
            depth,
            tint,
        }
    }
}

/// Compute monotonic isometric depth for a given tile coordinate, elevation, and layer.
///
/// Guarantees that tiles further in the background (smaller X+Y) have strictly smaller depth
/// than foreground tiles (larger X+Y), ensuring Painter's algorithm renders correctly.
pub fn calculate_isometric_depth(x: u16, y: u16, elevation: i16, layer: RenderLayer) -> f32 {
    let manhattan = (x as u32 + y as u32) as f32;
    let elev = elevation as f32;
    // Each diagonal diagonal rank is separated by 100.0 depth units
    manhattan * 100.0 + (elev * 5.0) + layer.offset()
}

/// Sort render items in-place using stable back-to-front depth ordering.
pub fn sort_render_items(items: &mut [RenderItem]) {
    items.sort_by(|a, b| {
        a.depth
            .partial_cmp(&b.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
