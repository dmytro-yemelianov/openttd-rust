use serde::{Deserialize, Serialize};
use std::fmt;
use transport_types::enum_::TileKind;
use transport_types::{StationID, TileIndex};

/// Errors that can occur during Map construction and validation
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum MapError {
    #[error("Invalid map dimensions: width and height must be non-zero (got {width}x{height})")]
    ZeroDimension { width: u16, height: u16 },
    #[error("Map allocation limit exceeded: {0} tiles exceeds maximum allowed")]
    AllocationTooLarge(u32),
    #[error("Tile index out of bounds: ({x}, {y})")]
    OutOfBounds { x: u16, y: u16 },
    #[error("Map tile buffer mismatch: expected {expected} tiles, got {actual}")]
    TileCountMismatch { expected: usize, actual: usize },
}

/// Size of the simulation map in tiles
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MapSize {
    pub width: u16,
    pub height: u16,
}

impl MapSize {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> u32 {
        u32::from(self.width) * u32::from(self.height)
    }

    pub fn is_valid_index(&self, index: TileIndex) -> bool {
        index.x < self.width && index.y < self.height
    }
}

/// Base tile record (16 bytes aligned)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TileBase {
    /// Kind of terrain/infrastructure
    pub kind: TileKind,
    /// Owner company (for infrastructure)
    pub owner: Option<transport_types::CompanyID>,
    /// Height above sea level (in millimeters?)
    pub height: i16,
    /// Local water height (for rivers, etc.)
    pub water_height: i16,
}

/// Tile extension record (4 bytes)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TileExtension {
    /// Additional data based on tile kind
    pub data: u32,
}

/// Complete tile record (20 bytes total: 16-byte base + 4-byte extension)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Tile {
    pub base: TileBase,
    pub extension: TileExtension,
}

impl Tile {
    pub fn new_empty() -> Self {
        Self {
            base: TileBase {
                kind: TileKind::Clear,
                owner: None,
                height: 0,
                water_height: 0,
            },
            extension: TileExtension { data: 0 },
        }
    }

    pub fn is_water(&self) -> bool {
        matches!(self.base.kind, TileKind::Water)
    }

    pub fn is_land(&self) -> bool {
        !self.is_water()
    }

    pub fn is_buildable(&self) -> bool {
        // Simplified: can build on clear, grass, etc.
        matches!(
            self.base.kind,
            TileKind::Clear | TileKind::Grass | TileKind::Water
        )
    }
}

/// 2D map storage
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Map {
    size: MapSize,
    tiles: Vec<Tile>,
}

impl Map {
    pub const MAX_TILES: u32 = 4096 * 4096; // 16M tiles max

    /// Try to construct a new map with dimension and allocation validation
    pub fn try_new(size: MapSize) -> Result<Self, MapError> {
        if size.width == 0 || size.height == 0 {
            return Err(MapError::ZeroDimension {
                width: size.width,
                height: size.height,
            });
        }
        let area = size.area();
        if area > Self::MAX_TILES {
            return Err(MapError::AllocationTooLarge(area));
        }
        let tile_count = area as usize;
        Ok(Self {
            size,
            tiles: vec![Tile::new_empty(); tile_count],
        })
    }

    /// Construct a new map, falling back safely to a 1x1 map on invalid dimensions
    pub fn new(size: MapSize) -> Self {
        Self::try_new(size).unwrap_or_else(|_| Self {
            size: MapSize::new(1, 1),
            tiles: vec![Tile::new_empty()],
        })
    }

    pub fn size(&self) -> MapSize {
        self.size
    }

    /// Validate map dimensions, bounds and tile count integrity
    pub fn validate(&self) -> Result<(), MapError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Err(MapError::ZeroDimension {
                width: self.size.width,
                height: self.size.height,
            });
        }
        let area = self.size.area();
        if area > Self::MAX_TILES {
            return Err(MapError::AllocationTooLarge(area));
        }
        if self.tiles.len() != area as usize {
            return Err(MapError::TileCountMismatch {
                expected: area as usize,
                actual: self.tiles.len(),
            });
        }
        Ok(())
    }

    /// Get linear index from tile coordinates
    fn index(&self, tile: TileIndex) -> Option<usize> {
        if !self.size.is_valid_index(tile) {
            return None;
        }
        Some(usize::from(tile.y) * usize::from(self.size.width) + usize::from(tile.x))
    }

    /// Get tile at coordinates (returns None if out of bounds)
    pub fn get(&self, tile: TileIndex) -> Option<&Tile> {
        self.index(tile).map(|i| &self.tiles[i])
    }

    /// Get mutable tile at coordinates
    pub fn get_mut(&mut self, tile: TileIndex) -> Option<&mut Tile> {
        self.index(tile).map(|i| &mut self.tiles[i])
    }

    /// Set tile at coordinates
    pub fn set(&mut self, tile: TileIndex, value: Tile) -> Result<(), &'static str> {
        if let Some(idx) = self.index(tile) {
            self.tiles[idx] = value;
            Ok(())
        } else {
            Err("Tile index out of bounds")
        }
    }

    /// Iterate over all tiles with their coordinates
    pub fn iter_enumerated(&self) -> impl Iterator<Item = (TileIndex, &Tile)> {
        self.tiles.iter().enumerate().map(|(idx, tile)| {
            let x = (idx % self.size.width as usize) as u16;
            let y = (idx / self.size.width as usize) as u16;
            (TileIndex::new(x, y), tile)
        })
    }

    /// Get station at tile (if any)
    pub fn station_at(&self, tile: TileIndex) -> Option<StationID> {
        self.get(tile).and_then(|t| {
            if matches!(t.base.kind, TileKind::Station) {
                Some(StationID(t.extension.data))
            } else {
                None
            }
        })
    }

    /// Set station at coordinates
    pub fn set_station_at(
        &mut self,
        tile: TileIndex,
        station_id: StationID,
        owner: Option<transport_types::CompanyID>,
    ) -> Result<(), &'static str> {
        if let Some(t) = self.get_mut(tile) {
            t.base.kind = TileKind::Station;
            t.base.owner = owner;
            t.extension.data = station_id.0;
            Ok(())
        } else {
            Err("Tile index out of bounds")
        }
    }

    /// Clear station at coordinates
    pub fn clear_station_at(&mut self, tile: TileIndex) -> Result<(), &'static str> {
        if let Some(t) = self.get_mut(tile) {
            if matches!(t.base.kind, TileKind::Station) {
                t.base.kind = if t.base.water_height > 0 {
                    TileKind::Water
                } else {
                    TileKind::Clear
                };
                t.base.owner = None;
                t.extension.data = 0;
            }
            Ok(())
        } else {
            Err("Tile index out of bounds")
        }
    }

    /// Check if tile is water (for ship movement)
    pub fn is_water_at(&self, tile: TileIndex) -> bool {
        self.get(tile).map(|t| t.is_water()).unwrap_or(false)
    }

    /// Check if tile is navigable by water vehicles (water or water station/dock)
    pub fn is_navigable_water(&self, tile: TileIndex) -> bool {
        self.get(tile)
            .map(|t| {
                t.is_water()
                    || (matches!(t.base.kind, TileKind::Station) && t.base.water_height > 0)
            })
            .unwrap_or(false)
    }

    /// Get adjacent tiles (4-directional)
    pub fn adjacent_tiles(&self, tile: TileIndex) -> [Option<TileIndex>; 4] {
        if !self.size.is_valid_index(tile) {
            return [None, None, None, None];
        }
        let x = i32::from(tile.x);
        let y = i32::from(tile.y);
        [
            if y > 0 {
                Some(TileIndex::new(x as u16, (y - 1) as u16))
            } else {
                None
            }, // North
            if x < i32::from(self.size.width) - 1 {
                Some(TileIndex::new((x + 1) as u16, y as u16))
            } else {
                None
            }, // East
            if y < i32::from(self.size.height) - 1 {
                Some(TileIndex::new(x as u16, (y + 1) as u16))
            } else {
                None
            }, // South
            if x > 0 {
                Some(TileIndex::new((x - 1) as u16, y as u16))
            } else {
                None
            }, // West
        ]
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Map {}x{}", self.size.width, self.size.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_creation() {
        let map = Map::new(MapSize::new(10, 10));
        assert_eq!(map.size().width, 10);
        assert_eq!(map.size().height, 10);
        assert_eq!(map.size().area(), 100);
    }

    #[test]
    fn test_tile_access() {
        let mut map = Map::new(MapSize::new(5, 5));
        let tile = TileIndex::new(2, 3);

        // Get should return Some for valid tile
        assert!(map.get(tile).is_some());

        // Set a tile
        let new_tile = Tile {
            base: TileBase {
                kind: TileKind::Water,
                owner: None,
                height: 0,
                water_height: 10,
            },
            extension: TileExtension { data: 0x12345678 },
        };
        assert!(map.set(tile, new_tile).is_ok());

        // Check it was set
        assert_eq!(map.get(tile).unwrap().base.kind, TileKind::Water);
        assert_eq!(map.get(tile).unwrap().extension.data, 0x12345678);

        // Out of bounds should return None
        assert!(map.get(TileIndex::new(10, 10)).is_none());
        assert!(map.set(TileIndex::new(10, 10), Tile::new_empty()).is_err());
    }

    #[test]
    fn test_map_validation_rejects_zero_dimensions() {
        assert_eq!(
            Map::try_new(MapSize::new(0, 10)),
            Err(MapError::ZeroDimension {
                width: 0,
                height: 10
            })
        );
        assert_eq!(
            Map::try_new(MapSize::new(10, 0)),
            Err(MapError::ZeroDimension {
                width: 10,
                height: 0
            })
        );
        assert_eq!(
            Map::try_new(MapSize::new(0, 0)),
            Err(MapError::ZeroDimension {
                width: 0,
                height: 0
            })
        );
    }

    #[test]
    fn test_map_validation_rejects_excessive_allocation() {
        assert_eq!(
            Map::try_new(MapSize::new(5000, 5000)),
            Err(MapError::AllocationTooLarge(25_000_000))
        );
    }

    #[test]
    fn test_adjacent_tiles_out_of_bounds_returns_none() {
        let map = Map::new(MapSize::new(10, 10));
        assert_eq!(
            map.adjacent_tiles(TileIndex::new(50, 50)),
            [None, None, None, None]
        );
    }

    #[test]
    fn test_checked_arithmetic_overflow_and_underflow() {
        use transport_types::unit::{CargoAmount, Money, Ticks};

        // Money overflow
        assert_eq!(Money::MAX.checked_add(Money(1)), None);
        assert_eq!(Money::MIN.checked_sub(Money(1)), None);

        // CargoAmount underflow
        let c1 = CargoAmount(10);
        let c2 = CargoAmount(20);
        assert_eq!(c1.checked_sub(c2), None);
        assert!(c1.sub_checked(c2).is_err());
        assert_eq!(c1.saturating_sub(c2), CargoAmount(0));

        // Ticks advance
        let mut t = Ticks(Ticks::MAX.0 - 1);
        assert!(t.advance_checked().is_ok());
        assert!(t.advance_checked().is_err());
    }

    #[test]
    fn test_tile_memory_layout_and_size() {
        use std::mem::size_of;
        let base_size = size_of::<TileBase>();
        let ext_size = size_of::<TileExtension>();
        let tile_size = size_of::<Tile>();
        println!("TileBase size: {base_size} bytes");
        println!("TileExtension size: {ext_size} bytes");
        println!("Tile size: {tile_size} bytes");
        assert_eq!(base_size, 16);
        assert_eq!(ext_size, 4);
        assert_eq!(tile_size, 20);
    }
}
