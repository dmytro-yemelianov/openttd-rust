use std::collections::{HashMap, HashSet, VecDeque};
use transport_types::TileIndex;
use transport_world::Map;

/// Identifier for an isolated or connected water body.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WaterBodyId(pub u32);

/// Metrics and spatial tiles belonging to a water body.
#[derive(Debug, Clone)]
pub struct WaterBodyInfo {
    pub id: WaterBodyId,
    pub tile_count: usize,
    pub is_navigable: bool,
    pub bounds: (TileIndex, TileIndex),
}

/// Topology analyzer for map waterways and landmasses.
pub struct TopologyAnalyzer {
    water_components: HashMap<TileIndex, WaterBodyId>,
    bodies: HashMap<WaterBodyId, WaterBodyInfo>,
}

impl TopologyAnalyzer {
    /// Minimum connected water tiles required for long-distance ship navigation.
    pub const DEFAULT_MIN_NAVIGABLE_TILES: usize = 25;

    /// Analyze water bodies and waterway connectivity across the map.
    pub fn analyze(map: &Map, min_navigable_tiles: usize) -> Self {
        let mut water_components = HashMap::new();
        let mut bodies = HashMap::new();
        let mut visited = HashSet::new();
        let mut next_id = 1u32;

        let width = map.size().width;
        let height = map.size().height;

        for y in 0..height {
            for x in 0..width {
                let start = TileIndex::new(x, y);
                if visited.contains(&start) {
                    continue;
                }

                if let Some(tile) = map.get(start) {
                    if !tile.is_water() {
                        continue;
                    }

                    // Flood-fill connected water body
                    let body_id = WaterBodyId(next_id);
                    next_id += 1;

                    let mut queue = VecDeque::new();
                    queue.push_back(start);
                    visited.insert(start);
                    water_components.insert(start, body_id);

                    let mut min_x = start.x;
                    let mut max_x = start.x;
                    let mut min_y = start.y;
                    let mut max_y = start.y;
                    let mut count = 0;

                    while let Some(current) = queue.pop_front() {
                        count += 1;
                        min_x = min_x.min(current.x);
                        max_x = max_x.max(current.x);
                        min_y = min_y.min(current.y);
                        max_y = max_y.max(current.y);

                        let neighbors = [
                            (current.x.wrapping_sub(1), current.y),
                            (current.x + 1, current.y),
                            (current.x, current.y.wrapping_sub(1)),
                            (current.x, current.y + 1),
                        ];

                        for &(nx, ny) in &neighbors {
                            if nx < width && ny < height {
                                let neighbor_idx = TileIndex::new(nx, ny);
                                if !visited.contains(&neighbor_idx) {
                                    if let Some(n_tile) = map.get(neighbor_idx) {
                                        if n_tile.is_water() {
                                            visited.insert(neighbor_idx);
                                            water_components.insert(neighbor_idx, body_id);
                                            queue.push_back(neighbor_idx);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    bodies.insert(
                        body_id,
                        WaterBodyInfo {
                            id: body_id,
                            tile_count: count,
                            is_navigable: count >= min_navigable_tiles,
                            bounds: (TileIndex::new(min_x, min_y), TileIndex::new(max_x, max_y)),
                        },
                    );
                }
            }
        }

        Self {
            water_components,
            bodies,
        }
    }

    /// Query the water body ID containing the tile.
    pub fn water_body_at(&self, tile: TileIndex) -> Option<WaterBodyId> {
        self.water_components.get(&tile).copied()
    }

    /// Check whether two water tiles belong to the same connected water body.
    pub fn are_water_tiles_connected(&self, a: TileIndex, b: TileIndex) -> bool {
        match (self.water_components.get(&a), self.water_components.get(&b)) {
            (Some(body_a), Some(body_b)) => body_a == body_b,
            _ => false,
        }
    }

    /// Check if a water tile is part of a navigable waterway (large enough for commercial ships).
    pub fn is_tile_navigable(&self, tile: TileIndex) -> bool {
        if let Some(body_id) = self.water_components.get(&tile) {
            if let Some(info) = self.bodies.get(body_id) {
                return info.is_navigable;
            }
        }
        false
    }

    /// Find all dry land tiles directly adjacent (4-way) to the specified water body.
    /// These are ideal candidates for docks and harbor stations.
    pub fn find_coastal_land_tiles(&self, map: &Map, body_id: WaterBodyId) -> Vec<TileIndex> {
        let width = map.size().width;
        let height = map.size().height;
        let mut coastal = HashSet::new();

        for (&water_tile, &b_id) in &self.water_components {
            if b_id != body_id {
                continue;
            }

            let neighbors = [
                (water_tile.x.wrapping_sub(1), water_tile.y),
                (water_tile.x + 1, water_tile.y),
                (water_tile.x, water_tile.y.wrapping_sub(1)),
                (water_tile.x, water_tile.y + 1),
            ];

            for &(nx, ny) in &neighbors {
                if nx < width && ny < height {
                    let neighbor_idx = TileIndex::new(nx, ny);
                    if let Some(tile) = map.get(neighbor_idx) {
                        if !tile.is_water() {
                            coastal.insert(neighbor_idx);
                        }
                    }
                }
            }
        }

        let mut sorted: Vec<_> = coastal.into_iter().collect();
        sorted.sort_by_key(|t| (t.y, t.x));
        sorted
    }

    /// Retrieve all identified water bodies.
    pub fn bodies(&self) -> &HashMap<WaterBodyId, WaterBodyInfo> {
        &self.bodies
    }
}
