use std::collections::HashMap;
use transport_types::{TileIndex, VehicleID};

use crate::backend::{Heading8, Vec2};
use crate::camera::Camera;

/// Historical vehicle state buffered for sub-tick visual interpolation.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleMotionState {
    pub prev_pos: TileIndex,
    pub prev_sub: (u8, u8),
    pub curr_pos: TileIndex,
    pub curr_sub: (u8, u8),
    pub heading: Heading8,
}

/// Sub-tick visual interpolation engine.
///
/// Smoothly blends vehicle positions, headings, and camera movement between discrete
/// simulation ticks, enabling 60 FPS, 120 FPS, or 144 FPS rendering without tick jitter.
#[derive(Debug, Default, Clone)]
pub struct FrameInterpolator {
    pub vehicles: HashMap<VehicleID, VehicleMotionState>,
}

impl FrameInterpolator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a vehicle position update from a completed simulation tick.
    pub fn update_vehicle(
        &mut self,
        vehicle_id: VehicleID,
        new_pos: TileIndex,
        new_sub: (u8, u8),
    ) {
        if let Some(entry) = self.vehicles.get_mut(&vehicle_id) {
            let dx = new_pos.x as i32 - entry.curr_pos.x as i32;
            let dy = new_pos.y as i32 - entry.curr_pos.y as i32;
            let heading = if dx != 0 || dy != 0 {
                Heading8::from_delta(dx, dy)
            } else {
                entry.heading
            };

            entry.prev_pos = entry.curr_pos;
            entry.prev_sub = entry.curr_sub;
            entry.curr_pos = new_pos;
            entry.curr_sub = new_sub;
            entry.heading = heading;
        } else {
            self.vehicles.insert(
                vehicle_id,
                VehicleMotionState {
                    prev_pos: new_pos,
                    prev_sub: new_sub,
                    curr_pos: new_pos,
                    curr_sub: new_sub,
                    heading: Heading8::North,
                },
            );
        }
    }

    /// Remove a decommissioned vehicle from the interpolation buffer.
    pub fn remove_vehicle(&mut self, vehicle_id: VehicleID) {
        self.vehicles.remove(&vehicle_id);
    }

    /// Calculate smooth screen position and heading for a vehicle at sub-tick blending factor `alpha`.
    ///
    /// `alpha` is in the range `[0.0, 1.0]`, representing fraction of tick elapsed.
    pub fn interpolate_vehicle(
        &self,
        vehicle_id: VehicleID,
        alpha: f32,
        camera: &Camera,
        elevation: i16,
    ) -> Option<(Vec2, Heading8)> {
        let state = self.vehicles.get(&vehicle_id)?;
        let prev_screen = camera.world_to_screen(state.prev_pos, state.prev_sub, elevation);
        let curr_screen = camera.world_to_screen(state.curr_pos, state.curr_sub, elevation);

        let interpolated_pos = prev_screen.lerp(curr_screen, alpha);
        Some((interpolated_pos, state.heading))
    }

    /// Smooth camera follow: interpolate camera offset towards a target point.
    pub fn smooth_follow(camera: &mut Camera, target_world: Vec2, speed: f32) {
        let s = speed.clamp(0.01, 1.0);
        camera.offset.x += (target_world.x - camera.offset.x) * s;
        camera.offset.y += (target_world.y - camera.offset.y) * s;
    }
}
