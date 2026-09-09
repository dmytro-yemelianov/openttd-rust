use serde::{Deserialize, Serialize};
use std::fmt;
use crate::id::{StationID, TileIndex};
use crate::transport_types::unit::Coord;
use crate::transport_types::enum_::TileKind;

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

/// Base tile record (8 bytes)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TileBase {
    /// Kind of terrain/infrastructure
    pub kind: TileKind,
    /// Owner company (for infrastructure)
    pub owner: Option<id::CompanyID>,
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

/// Complete tile record (12 bytes total)
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
    pub fn new(size: MapSize) -> Self {
        let tile_count = size.area() as usize;
        Self {
            size,
            tiles: vec![Tile::new_empty(); tile_count],
        }
    }

    pub fn size(&self) -> MapSize {
        self.size
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
        // This would normally look up from a station map
        // For now, we'll store station locations separately
        None
    }

    /// Check if tile is water (for ship movement)
    pub fn is_water_at(&self, tile: TileIndex) -> bool {
        self.get(tile).map(|t| t.is_water()).unwrap_or(false)
    }

    /// Get adjacent tiles (4-directional)
    pub fn adjacent_tiles(&self, tile: TileIndex) -> [Option<TileIndex>; 4] {
        let x = i32::from(tile.x);
        let y = i32::from(tile.y);
        [
            if y > 0 { Some(TileIndex::new(x as u16, (y - 1) as u16)) } else { None }, // North
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
}