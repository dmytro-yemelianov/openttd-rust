use std::collections::HashSet;
use transport_sim::World;
use transport_types::enum_::TileKind;
use transport_types::{CompanyID, EngineID, Money, TileIndex, TownID};

/// Result of a capability-aware dry-run command validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementValidity {
    /// Valid placement with estimated construction or purchase cost
    Valid { cost: Money },
    /// Selected tile is outside the valid map dimensions
    OutOfBounds(TileIndex),
    /// Tile is already occupied by a station, building, or existing infrastructure
    TileOccupied(TileIndex),
    /// Terrain is not suitable for this structure (e.g. building a dock on land, or track on deep water)
    InvalidTerrain { tile: TileIndex, kind: TileKind },
    /// Company has insufficient funds to afford the operation
    InsufficientFunds { cost: Money, available: Money },
    /// Company was not found in the world
    CompanyNotFound(CompanyID),
}

impl PlacementValidity {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }

    pub fn cost(&self) -> Money {
        match self {
            Self::Valid { cost } => *cost,
            Self::InsufficientFunds { cost, .. } => *cost,
            _ => Money(0),
        }
    }
}

/// Catchment area evaluation summarizing population and employment served by a station.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchmentSummary {
    pub catchment_tiles: HashSet<TileIndex>,
    pub residential_capacity: u32,
    pub workplace_capacity: u32,
    pub towns_served: Vec<TownID>,
}

/// Dry-run validation engine providing single-source-of-truth placement checks and catchment overlays.
pub struct DryRunValidator;

impl DryRunValidator {
    /// Validate station dock construction across a list of candidate tiles.
    pub fn validate_station_placement(
        world: &World,
        company_id: CompanyID,
        tiles: &[TileIndex],
    ) -> PlacementValidity {
        let company = match world.companies.get(&company_id) {
            Some(c) => c,
            None => return PlacementValidity::CompanyNotFound(company_id),
        };

        if tiles.is_empty() {
            return PlacementValidity::OutOfBounds(TileIndex::new(0, 0));
        }

        let map_size = world.map.size();
        for &t in tiles {
            if !map_size.is_valid_index(t) {
                return PlacementValidity::OutOfBounds(t);
            }

            if world.tile_to_station.contains_key(&t) || world.map.station_at(t).is_some() {
                return PlacementValidity::TileOccupied(t);
            }

            // Must be built on buildable terrain (water/clear/grass)
            if let Some(tile) = world.map.get(t) {
                if !tile.is_buildable() {
                    return PlacementValidity::InvalidTerrain {
                        tile: t,
                        kind: tile.base.kind,
                    };
                }
            }
        }

        let cost = Money(1000 * tiles.len() as i64);
        if company.money.0 < cost.0 {
            return PlacementValidity::InsufficientFunds {
                cost,
                available: company.money,
            };
        }

        PlacementValidity::Valid { cost }
    }

    /// Validate vehicle purchase and deployment at a specific tile.
    pub fn validate_vehicle_placement(
        world: &World,
        company_id: CompanyID,
        engine_id: EngineID,
        position: TileIndex,
    ) -> PlacementValidity {
        let company = match world.companies.get(&company_id) {
            Some(c) => c,
            None => return PlacementValidity::CompanyNotFound(company_id),
        };

        if !world.map.size().is_valid_index(position) {
            return PlacementValidity::OutOfBounds(position);
        }

        let cost = world
            .engines
            .get(&engine_id)
            .map(|e| e.cost)
            .unwrap_or(Money(5000));

        if company.money.0 < cost.0 {
            return PlacementValidity::InsufficientFunds {
                cost,
                available: company.money,
            };
        }

        PlacementValidity::Valid { cost }
    }

    /// Calculate the catchment area (walking radius <= 3 Manhattan distance) for a set of station tiles.
    pub fn calculate_catchment(world: &World, station_tiles: &[TileIndex]) -> CatchmentSummary {
        let mut catchment_tiles = HashSet::new();
        let map_size = world.map.size();

        for &st in station_tiles {
            let tx = st.x as i32;
            let ty = st.y as i32;

            for dy in -3..=3i32 {
                let max_dx = 3 - dy.abs();
                for dx in -max_dx..=max_dx {
                    let nx = tx + dx;
                    let ny = ty + dy;
                    if nx >= 0 && ny >= 0 && nx < map_size.width as i32 && ny < map_size.height as i32 {
                        catchment_tiles.insert(TileIndex::new(nx as u16, ny as u16));
                    }
                }
            }
        }

        let mut residential_capacity = 0u32;
        let mut workplace_capacity = 0u32;
        let mut towns = HashSet::new();

        for bldg in world.buildings.values() {
            if catchment_tiles.contains(&bldg.tile) {
                towns.insert(bldg.town_id);
                if bldg.is_residence() {
                    residential_capacity += bldg.capacity as u32;
                } else if bldg.is_workplace() {
                    workplace_capacity += bldg.capacity as u32;
                }
            }
        }

        let mut towns_served: Vec<TownID> = towns.into_iter().collect();
        towns_served.sort_unstable();

        CatchmentSummary {
            catchment_tiles,
            residential_capacity,
            workplace_capacity,
            towns_served,
        }
    }
}
