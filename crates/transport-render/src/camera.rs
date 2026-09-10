use serde::{Deserialize, Serialize};
use transport_types::TileIndex;

use crate::backend::Vec2;

/// Geometric projection mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectionMode {
    /// Classical 2:1 dimetric isometric projection (OpenTTD / OpenRCT2 standard).
    Isometric,
    /// Top-down 2D orthographic projection.
    Orthographic,
}

/// 2D/Isometric camera managing viewport scrolling, zoom levels, and frustum culling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub projection: ProjectionMode,
    pub offset: Vec2,
    pub zoom: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub tile_width: f32,
    pub tile_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(800.0, 600.0, ProjectionMode::Isometric)
    }
}

impl Camera {
    pub fn new(viewport_width: f32, viewport_height: f32, projection: ProjectionMode) -> Self {
        Self {
            projection,
            offset: Vec2::ZERO,
            zoom: 1.0,
            viewport_width,
            viewport_height,
            tile_width: 64.0,
            tile_height: 32.0,
        }
    }

    /// Set camera zoom level clamped to safe visible bounds (0.25x to 4.0x).
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.25, 4.0);
    }

    /// Pan camera offset by screen delta.
    pub fn pan(&mut self, delta: Vec2) {
        self.offset.x += delta.x;
        self.offset.y += delta.y;
    }

    /// Transform a world tile coordinate, sub-tile offset, and terrain elevation into 2D screen coordinates.
    pub fn world_to_screen(&self, tile: TileIndex, sub_tile: (u8, u8), elevation: i16) -> Vec2 {
        let fx = tile.x as f32 + (sub_tile.0 as f32 / 16.0);
        let fy = tile.y as f32 + (sub_tile.1 as f32 / 16.0);
        let felev = elevation as f32;

        match self.projection {
            ProjectionMode::Isometric => {
                let half_w = (self.tile_width * self.zoom) / 2.0;
                let half_h = (self.tile_height * self.zoom) / 2.0;
                let elev_scale = 8.0 * self.zoom;

                let sx = (fx - fy) * half_w + self.offset.x + (self.viewport_width / 2.0);
                let sy = (fx + fy) * half_h - (felev * elev_scale) + self.offset.y;

                Vec2::new(sx, sy)
            }
            ProjectionMode::Orthographic => {
                let cell_w = self.tile_width * self.zoom;
                let cell_h = self.tile_height * self.zoom;

                let sx = fx * cell_w + self.offset.x;
                let sy = fy * cell_h + self.offset.y;

                Vec2::new(sx, sy)
            }
        }
    }

    /// Inverse projection: transform screen coordinates back into the ground-plane world TileIndex.
    pub fn screen_to_world(&self, screen: Vec2) -> Option<TileIndex> {
        match self.projection {
            ProjectionMode::Isometric => {
                let half_w = (self.tile_width * self.zoom) / 2.0;
                let half_h = (self.tile_height * self.zoom) / 2.0;

                if half_w <= 0.0 || half_h <= 0.0 {
                    return None;
                }

                let adj_x = screen.x - self.offset.x - (self.viewport_width / 2.0);
                let adj_y = screen.y - self.offset.y;

                let fx = (adj_x / half_w + adj_y / half_h) / 2.0;
                let fy = (adj_y / half_h - adj_x / half_w) / 2.0;

                if fx >= 0.0 && fy >= 0.0 && fx < u16::MAX as f32 && fy < u16::MAX as f32 {
                    Some(TileIndex::new(fx.floor() as u16, fy.floor() as u16))
                } else {
                    None
                }
            }
            ProjectionMode::Orthographic => {
                let cell_w = self.tile_width * self.zoom;
                let cell_h = self.tile_height * self.zoom;

                if cell_w <= 0.0 || cell_h <= 0.0 {
                    return None;
                }

                let fx = (screen.x - self.offset.x) / cell_w;
                let fy = (screen.y - self.offset.y) / cell_h;

                if fx >= 0.0 && fy >= 0.0 && fx < u16::MAX as f32 && fy < u16::MAX as f32 {
                    Some(TileIndex::new(fx.floor() as u16, fy.floor() as u16))
                } else {
                    None
                }
            }
        }
    }

    /// Compute visible tile bounding box for frustum culling.
    ///
    /// Returns `(min_tile, max_tile)` clamped to the map dimensions.
    pub fn visible_tile_bounds(&self, map_width: u16, map_height: u16) -> (TileIndex, TileIndex) {
        let corners = [
            Vec2::new(0.0, 0.0),
            Vec2::new(self.viewport_width, 0.0),
            Vec2::new(0.0, self.viewport_height),
            Vec2::new(self.viewport_width, self.viewport_height),
        ];

        let mut min_x = u16::MAX;
        let mut min_y = u16::MAX;
        let mut max_x = 0;
        let mut max_y = 0;

        for &corner in &corners {
            if let Some(t) = self.screen_to_world(corner) {
                min_x = min_x.min(t.x);
                min_y = min_y.min(t.y);
                max_x = max_x.max(t.x);
                max_y = max_y.max(t.y);
            }
        }

        // Expand padding by 2 tiles to avoid edge clipping of multi-tile or elevated sprites
        let p_min_x = min_x.saturating_sub(2);
        let p_min_y = min_y.saturating_sub(2);
        let p_max_x = (max_x.saturating_add(2)).min(map_width.saturating_sub(1));
        let p_max_y = (max_y.saturating_add(2)).min(map_height.saturating_sub(1));

        (
            TileIndex::new(p_min_x, p_min_y),
            TileIndex::new(p_max_x, p_max_y),
        )
    }
}
