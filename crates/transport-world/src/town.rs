use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use transport_types::enum_::TileKind;
use transport_types::{BuildingID, TileIndex, TownID};

use crate::map::Map;

/// Functional classification of an urban building.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingKind {
    /// Housing dwelling where residents reside and generate commute trips
    Residential,
    /// Office or retail destination providing service employment
    Commercial,
    /// Factory or processing facility providing production employment
    Industrial,
}

/// A physical building in a town.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Building {
    pub id: BuildingID,
    pub town_id: TownID,
    pub tile: TileIndex,
    pub kind: BuildingKind,
    pub capacity: u16,
    pub occupancy: u16,
}

impl Building {
    pub fn new(
        id: BuildingID,
        town_id: TownID,
        tile: TileIndex,
        kind: BuildingKind,
        capacity: u16,
    ) -> Self {
        Self {
            id,
            town_id,
            tile,
            kind,
            capacity,
            occupancy: capacity, // Starts fully populated
        }
    }

    /// Returns true if this building can accept worker commuters.
    pub fn is_workplace(&self) -> bool {
        matches!(self.kind, BuildingKind::Commercial | BuildingKind::Industrial)
    }

    /// Returns true if this building generates home commuters.
    pub fn is_residence(&self) -> bool {
        matches!(self.kind, BuildingKind::Residential)
    }
}

/// Macro-level urban center governing population, buildings, and growth pulses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Town {
    pub id: TownID,
    pub name: String,
    pub center: TileIndex,
    /// Total population residing in this town
    pub census_population: u32,
    /// Average satisfaction score (0 = furious, 100 = delighted)
    pub satisfaction_score: u8,
    /// Total cumulative commute trips requested by town residents
    pub demanded_commutes: u64,
    /// Total cumulative commute trips successfully completed
    pub fulfilled_commutes: u64,
    /// Registered building IDs belonging to this town
    pub building_ids: Vec<BuildingID>,
    /// Accumulated growth momentum counter
    pub growth_counter: u32,
}

impl Town {
    pub fn new(id: TownID, name: String, center: TileIndex) -> Self {
        Self {
            id,
            name,
            center,
            census_population: 0,
            satisfaction_score: 80, // Default contentment
            demanded_commutes: 0,
            fulfilled_commutes: 0,
            building_ids: Vec::new(),
            growth_counter: 0,
        }
    }

    /// Ratio of fulfilled commutes to demanded commutes (0.0 to 1.0).
    pub fn fulfillment_ratio(&self) -> f64 {
        if self.demanded_commutes == 0 {
            1.0
        } else {
            self.fulfilled_commutes as f64 / self.demanded_commutes as f64
        }
    }

    /// Record the outcome of a commute trip and update rolling satisfaction.
    pub fn record_commute(&mut self, fulfilled: bool, peep_satisfaction: u8) {
        self.demanded_commutes += 1;
        if fulfilled {
            self.fulfilled_commutes += 1;
        }

        // Exponential moving average update for satisfaction score: 90% old + 10% new
        let old = self.satisfaction_score as u32;
        let incoming = peep_satisfaction as u32;
        self.satisfaction_score = ((old * 9 + incoming) / 10).min(100) as u8;
    }

    /// Evaluate whether this town qualifies for a growth pulse.
    ///
    /// Requires fulfillment ratio >= 60% and satisfaction >= 70.
    /// Returns a new `Building` to be constructed if growth occurs.
    pub fn evaluate_growth(
        &mut self,
        map: &mut Map,
        next_building_id: BuildingID,
        existing_buildings: &HashMap<BuildingID, Building>,
    ) -> Option<Building> {
        let ratio = self.fulfillment_ratio();

        // Growth threshold check (eliminates single-bus hacks)
        if ratio >= 0.60 && self.satisfaction_score >= 70 {
            self.growth_counter += 1;
        } else if ratio < 0.20 {
            self.growth_counter = self.growth_counter.saturating_sub(1);
        }

        // Trigger growth pulse every 5 sustained growth cycles
        if self.growth_counter >= 5 {
            self.growth_counter = 0;

            // Find an adjacent buildable tile radiating from the center
            if let Some(candidate_tile) = self.find_expansion_tile(map, existing_buildings) {
                // Alternate between Residential and Commercial/Industrial
                let building_kind = if self.building_ids.len() % 2 == 0 {
                    BuildingKind::Residential
                } else if self.building_ids.len() % 4 == 1 {
                    BuildingKind::Commercial
                } else {
                    BuildingKind::Industrial
                };

                let capacity = match building_kind {
                    BuildingKind::Residential => 20,
                    BuildingKind::Commercial => 15,
                    BuildingKind::Industrial => 25,
                };

                // Place building on map
                if let Some(tile) = map.get_mut(candidate_tile) {
                    tile.base.kind = match building_kind {
                        BuildingKind::Residential | BuildingKind::Commercial => TileKind::House,
                        BuildingKind::Industrial => TileKind::Industry,
                    };
                }

                let new_building = Building::new(
                    next_building_id,
                    self.id,
                    candidate_tile,
                    building_kind,
                    capacity,
                );

                self.building_ids.push(next_building_id);
                self.census_population += capacity as u32;

                return Some(new_building);
            }
        }

        None
    }

    /// Search for an available clear/grass tile within expanding radius of the town center.
    fn find_expansion_tile(
        &self,
        map: &Map,
        existing_buildings: &HashMap<BuildingID, Building>,
    ) -> Option<TileIndex> {
        let occupied_tiles: std::collections::HashSet<TileIndex> =
            existing_buildings.values().map(|b| b.tile).collect();

        let cx = self.center.x as i32;
        let cy = self.center.y as i32;

        // Radiate outwards from center in radius 1..=8
        for r in 1..=8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = cx + dx;
                    let ny = cy + dy;

                    if nx >= 0 && ny >= 0 && nx <= u16::MAX as i32 && ny <= u16::MAX as i32 {
                        let candidate = TileIndex::new(nx as u16, ny as u16);
                        if !map.size().is_valid_index(candidate) {
                            continue;
                        }
                        if occupied_tiles.contains(&candidate) {
                            continue;
                        }

                        if let Some(tile) = map.get(candidate) {
                            if matches!(tile.base.kind, TileKind::Clear | TileKind::Grass) {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }
        }

        None
    }
}
