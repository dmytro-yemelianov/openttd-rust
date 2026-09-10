use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use transport_types::enum_::{OrderType, TileKind, VehicleState};
use transport_types::{ServiceId, ServicePriority, TileIndex, VehicleID};
use transport_world::map::{Map, MapSize};

use crate::kernel::context::KernelContext;
use crate::kernel::driver::{DriverError, SubsystemDriver};
use crate::kernel::phase::Phase;

/// Passability mode determining which tiles are navigable by watercraft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassabilityMode {
    /// Strictly water tiles (Water, Station docks, Oil rigs) and the target station tile.
    StrictWater,
    /// Treats land tiles (Clear, Grass, House, Industry, Trees, Rail, Road) as impassable obstacles.
    AvoidObstacles,
}

/// Result of a budgeted pathfinding search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathResult {
    /// Successfully found a complete path from origin to destination (excluding origin, including target).
    Found(Vec<TileIndex>),
    /// Destination is completely unreachable by navigable tiles.
    Unreachable,
    /// Search reached the node expansion budget before reaching destination.
    BudgetExceeded,
}

/// Reusable, allocation-free buffers for high-performance grid A* search.
pub struct SearchBuffers {
    visited_gen: Vec<u32>,
    came_from: Vec<TileIndex>,
    g_score: Vec<u32>,
    current_generation: u32,
    size: MapSize,
}

impl SearchBuffers {
    pub fn new(size: MapSize) -> Self {
        let area = size.area() as usize;
        Self {
            visited_gen: vec![0; area],
            came_from: vec![TileIndex::new(0, 0); area],
            g_score: vec![u32::MAX; area],
            current_generation: 1,
            size,
        }
    }

    pub fn ensure_capacity(&mut self, size: MapSize) {
        if self.size != size {
            let area = size.area() as usize;
            self.visited_gen.resize(area, 0);
            self.came_from.resize(area, TileIndex::new(0, 0));
            self.g_score.resize(area, u32::MAX);
            self.size = size;
        }
    }

    pub fn start_search(&mut self) {
        self.current_generation = self.current_generation.wrapping_add(1);
        if self.current_generation == 0 {
            // In the rare event of u32 overflow, reset visited buffer
            self.visited_gen.fill(0);
            self.current_generation = 1;
        }
    }

    #[inline(always)]
    fn index(&self, tile: TileIndex) -> usize {
        (tile.y as usize) * (self.size.width as usize) + (tile.x as usize)
    }

    #[inline(always)]
    pub fn is_visited(&self, tile: TileIndex) -> bool {
        self.visited_gen[self.index(tile)] == self.current_generation
    }

    #[inline(always)]
    pub fn mark_visited(&mut self, tile: TileIndex, g: u32, parent: TileIndex) {
        let idx = self.index(tile);
        self.visited_gen[idx] = self.current_generation;
        self.g_score[idx] = g;
        self.came_from[idx] = parent;
    }

    #[inline(always)]
    pub fn get_g(&self, tile: TileIndex) -> u32 {
        if self.is_visited(tile) {
            self.g_score[self.index(tile)]
        } else {
            u32::MAX
        }
    }

    #[inline(always)]
    pub fn get_parent(&self, tile: TileIndex) -> TileIndex {
        self.came_from[self.index(tile)]
    }
}

/// Budgeted water pathfinding solver running during `Phase::Solvers`.
pub struct WaterPathSolver {
    pub passability_mode: PassabilityMode,
    pub budget_per_vehicle: usize,
    pub cached_routes: BTreeMap<VehicleID, (TileIndex, Vec<TileIndex>)>,
    buffers: SearchBuffers,
}

impl WaterPathSolver {
    /// Create a new solver with a default expansion budget of 512 nodes per vehicle.
    pub fn new() -> Self {
        Self {
            passability_mode: PassabilityMode::StrictWater,
            budget_per_vehicle: 512,
            cached_routes: BTreeMap::new(),
            buffers: SearchBuffers::new(MapSize::new(1, 1)),
        }
    }

    /// Configure passability mode (StrictWater vs AvoidObstacles).
    pub fn with_passability_mode(mut self, mode: PassabilityMode) -> Self {
        self.passability_mode = mode;
        self
    }

    /// Configure maximum node expansions per vehicle per tick.
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.budget_per_vehicle = budget;
        self
    }

    /// Check if a tile is passable for navigation.
    pub fn is_passable(&self, tile: TileIndex, target: TileIndex, map: &Map) -> bool {
        if tile == target {
            return true;
        }
        let Some(t) = map.get(tile) else {
            return false;
        };
        match self.passability_mode {
            PassabilityMode::StrictWater => {
                matches!(
                    t.base.kind,
                    TileKind::Water | TileKind::Station | TileKind::OilRig
                )
            }
            PassabilityMode::AvoidObstacles => {
                !matches!(
                    t.base.kind,
                    TileKind::Clear
                        | TileKind::Grass
                        | TileKind::House
                        | TileKind::Industry
                        | TileKind::Trees
                        | TileKind::Rail
                        | TileKind::Road
                )
            }
        }
    }

    /// Find an obstacle-avoiding path using budgeted A*.
    pub fn find_path(
        &mut self,
        start: TileIndex,
        target: TileIndex,
        map: &Map,
        budget: usize,
    ) -> PathResult {
        if start == target {
            return PathResult::Found(Vec::new());
        }

        self.buffers.ensure_capacity(map.size());
        self.buffers.start_search();

        // Priority Queue: (f_score, g_score, tile)
        let mut open_set: BinaryHeap<Reverse<(u32, u32, TileIndex)>> = BinaryHeap::new();

        let h_start = manhattan_distance(start, target);
        self.buffers.mark_visited(start, 0, start);
        open_set.push(Reverse((h_start, 0, start)));

        let mut expansions = 0;
        let map_size = map.size();

        while let Some(Reverse((_, current_g, current_tile))) = open_set.pop() {
            if current_tile == target {
                // Reconstruct path backwards from target to start
                let mut path = Vec::new();
                let mut curr = target;
                while curr != start {
                    path.push(curr);
                    curr = self.buffers.get_parent(curr);
                }
                path.reverse();
                return PathResult::Found(path);
            }

            expansions += 1;
            if expansions >= budget {
                return PathResult::BudgetExceeded;
            }

            // Expand 4 orthogonal cardinal neighbors
            let neighbors = [
                if current_tile.y > 0 {
                    Some(TileIndex::new(current_tile.x, current_tile.y - 1))
                } else {
                    None
                },
                if current_tile.y + 1 < map_size.height {
                    Some(TileIndex::new(current_tile.x, current_tile.y + 1))
                } else {
                    None
                },
                if current_tile.x > 0 {
                    Some(TileIndex::new(current_tile.x - 1, current_tile.y))
                } else {
                    None
                },
                if current_tile.x + 1 < map_size.width {
                    Some(TileIndex::new(current_tile.x + 1, current_tile.y))
                } else {
                    None
                },
            ];

            let next_g = current_g.saturating_add(1);

            for candidate in neighbors.into_iter().flatten() {
                if !self.is_passable(candidate, target, map) {
                    continue;
                }

                if next_g < self.buffers.get_g(candidate) {
                    self.buffers.mark_visited(candidate, next_g, current_tile);
                    let h = manhattan_distance(candidate, target);
                    let f = next_g.saturating_add(h);
                    open_set.push(Reverse((f, next_g, candidate)));
                }
            }
        }

        PathResult::Unreachable
    }
}

impl Default for WaterPathSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for WaterPathSolver {
    fn id(&self) -> ServiceId {
        ServiceId::Pathfinding
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::HIGH
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Solvers {
            return Ok(());
        }

        self.buffers.ensure_capacity(ctx.world.map.size());

        for i in 0..ctx.vehicle_order_buffer.len() {
            let vehicle_id = ctx.vehicle_order_buffer[i];
            let (vehicle_pos, orders_id, current_order_opt, is_traveling) = {
                let v = match ctx.world.vehicles.get(&vehicle_id) {
                    Some(v) => v,
                    None => continue,
                };
                (
                    v.position,
                    v.orders,
                    v.current_order,
                    v.state != VehicleState::Loading,
                )
            };

            if !is_traveling {
                self.cached_routes.remove(&vehicle_id);
                if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.next_waypoint = None;
                }
                continue;
            }

            let orders = match ctx.world.order_lists.get(&orders_id) {
                Some(list) if !list.is_empty() => list,
                _ => continue,
            };
            let orders_len = orders.len();

            let current_idx = current_order_opt
                .map(|idx| idx.0 as usize)
                .unwrap_or(0)
                % orders_len;

            let target_station_id = match orders.get(current_idx) {
                Some(OrderType::GoToStation { station_id, .. }) => *station_id,
                _ => {
                    self.cached_routes.remove(&vehicle_id);
                    if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                        v.next_waypoint = None;
                    }
                    continue;
                }
            };

            let target_tile = match ctx.world.stations.get(&target_station_id) {
                Some(s) if !s.tiles.is_empty() => s.tiles[0],
                _ => continue,
            };

            if vehicle_pos == target_tile {
                self.cached_routes.remove(&vehicle_id);
                if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.next_waypoint = None;
                }
                continue;
            }

            // Check if cached route is valid
            let mut need_new_path = true;
            if let Some((cached_target, route)) = self.cached_routes.get_mut(&vehicle_id) {
                if *cached_target == target_tile && !route.is_empty() {
                    // Advance past waypoints already reached
                    while !route.is_empty() && route[0] == vehicle_pos {
                        route.remove(0);
                    }
                    if let Some(&next) = route.first() {
                        if is_cardinal_adjacent(vehicle_pos, next) {
                            if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                                v.next_waypoint = Some(next);
                            }
                            need_new_path = false;
                        }
                    }
                }
            }

            if need_new_path {
                let budget = self.budget_per_vehicle;
                let search_res = self.find_path(vehicle_pos, target_tile, &ctx.world.map, budget);
                if let PathResult::Found(path) = search_res {
                    if let Some(&first_step) = path.first() {
                        if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                            v.next_waypoint = Some(first_step);
                        }
                        self.cached_routes.insert(vehicle_id, (target_tile, path));
                    }
                } else if let Some(v) = ctx.world.vehicles.get_mut(&vehicle_id) {
                    v.next_waypoint = None;
                }
            }
        }

        Ok(())
    }
}

/// Calculate Manhattan distance between two tiles.
#[inline(always)]
pub fn manhattan_distance(a: TileIndex, b: TileIndex) -> u32 {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs();
    let dy = (a.y as i32 - b.y as i32).unsigned_abs();
    dx + dy
}

/// Check if two tiles are orthogonally adjacent.
#[inline(always)]
pub fn is_cardinal_adjacent(a: TileIndex, b: TileIndex) -> bool {
    let dx = (a.x as i32 - b.x as i32).abs();
    let dy = (a.y as i32 - b.y as i32).abs();
    (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
}
