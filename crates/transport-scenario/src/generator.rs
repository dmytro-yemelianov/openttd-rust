use transport_sim::World;
use transport_types::enum_::TileKind;
use transport_types::unit::Money;
use transport_types::{BuildingID, CompanyID, TileIndex, TownID};
use transport_world::town::{Building, BuildingKind, Town};
use transport_world::{Company, Map, MapSize, Tile, TileBase, TileExtension};

/// Pure deterministic pseudo-random number generator (Xoshiro256**).
/// Guarantees bit-identical output across x86-64, ARM64, and WebAssembly.
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    s: [u64; 4],
}

impl DeterministicRng {
    /// Seed the generator using SplitMix64 initialization.
    pub fn seed_from_u64(seed: u64) -> Self {
        let mut sm = seed;
        let mut next_sm = || {
            sm = sm.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        };

        let mut s = [next_sm(), next_sm(), next_sm(), next_sm()];
        if s == [0, 0, 0, 0] {
            s[0] = 1;
        }
        Self { s }
    }

    /// Generate the next 64-bit pseudo-random unsigned integer.
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.s[1].wrapping_mul(5))
            .rotate_left(7)
            .wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    /// Generate the next 32-bit pseudo-random unsigned integer.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Return a random integer in `[0, bound)`.
    pub fn gen_range(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % (bound as u64)) as u32
    }
}

/// Configuration options for procedural map generation.
#[derive(Debug, Clone)]
pub struct MapGenConfig {
    pub size: MapSize,
    pub seed: u64,
    pub sea_level: u16,
    pub num_towns: u32,
    pub starting_capital: Money,
}

impl Default for MapGenConfig {
    fn default() -> Self {
        Self {
            size: MapSize::new(64, 64),
            seed: 42,
            sea_level: 4,
            num_towns: 4,
            starting_capital: Money(100_000),
        }
    }
}

/// Fixed-point integer 2D noise generator for reproducible terrain.
pub struct IntegerNoise2D {
    perm: [u8; 512],
}

impl IntegerNoise2D {
    pub fn new(rng: &mut DeterministicRng) -> Self {
        let mut p = [0u8; 256];
        for (i, item) in p.iter_mut().enumerate() {
            *item = i as u8;
        }
        // Fisher-Yates shuffle
        for i in (1..256).rev() {
            let j = rng.gen_range((i + 1) as u32) as usize;
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i % 256];
        }
        Self { perm }
    }

    /// Returns a fixed-point noise sample in range [0, 255] using integer lerp.
    pub fn sample(&self, x: u16, y: u16, scale: u16) -> u16 {
        if scale == 0 {
            return 128;
        }
        let xi = ((x / scale) % 256) as usize;
        let yi = ((y / scale) % 256) as usize;

        let xf = ((x % scale) as u32 * 256) / scale as u32;
        let yf = ((y % scale) as u32 * 256) / scale as u32;

        let aa = self.perm[self.perm[xi] as usize + yi] as u32;
        let ab = self.perm[self.perm[xi] as usize + (yi + 1)] as u32;
        let ba = self.perm[self.perm[xi + 1] as usize + yi] as u32;
        let bb = self.perm[self.perm[xi + 1] as usize + (yi + 1)] as u32;

        // Bilinear interpolation with integer weights
        let x1 = aa + ((ba.wrapping_sub(aa)).wrapping_mul(xf) >> 8);
        let x2 = ab + ((bb.wrapping_sub(ab)).wrapping_mul(xf) >> 8);
        let val = x1 + ((x2.wrapping_sub(x1)).wrapping_mul(yf) >> 8);

        (val & 0xFF) as u16
    }
}

/// Deterministic map generator.
pub struct MapGenerator;

impl MapGenerator {
    /// Generate a fully populated `World` from the given configuration.
    pub fn generate(config: &MapGenConfig) -> World {
        let mut rng = DeterministicRng::seed_from_u64(config.seed);
        let noise = IntegerNoise2D::new(&mut rng);

        let mut map = Map::new(config.size);
        let width = config.size.width;
        let height = config.size.height;

        // 1. Generate terrain heightmap
        for y in 0..height {
            for x in 0..width {
                // Multi-octave composite integer noise
                let n1 = noise.sample(x, y, 16);
                let n2 = noise.sample(x, y, 8);
                let n3 = noise.sample(x, y, 4);
                let combined = (n1 * 4 + n2 * 2 + n3) / 7; // [0, 255]
                let elevation = (combined * 16) / 256; // [0, 15]

                let tile = if elevation <= config.sea_level {
                    Tile {
                        base: TileBase {
                            kind: TileKind::Water,
                            owner: None,
                            height: 0,
                            water_height: 1,
                        },
                        extension: TileExtension { data: 0 },
                    }
                } else {
                    Tile {
                        base: TileBase {
                            kind: TileKind::Grass,
                            owner: None,
                            height: (elevation - config.sea_level) as i16,
                            water_height: 0,
                        },
                        extension: TileExtension { data: 0 },
                    }
                };
                let _ = map.set(TileIndex::new(x, y), tile);
            }
        }

        // 2. Initialize World
        let mut world = World::new(config.size);
        world.map = map;

        // 3. Place Towns
        let mut town_id_counter = 1u32;
        let mut building_id_counter = 1u32;

        let mut placed_towns = 0;
        let max_attempts = config.num_towns * 50;
        let mut attempts = 0;

        let town_names = [
            "Seaside Port",
            "Hilltop Valley",
            "Riverford",
            "Greenborough",
            "Lakeshire",
            "Sunken Point",
            "Ironridge",
            "Oakwood",
        ];

        while placed_towns < config.num_towns && attempts < max_attempts {
            attempts += 1;
            let cx = (rng.gen_range((width - 10) as u32) + 5) as u16;
            let cy = (rng.gen_range((height - 10) as u32) + 5) as u16;
            let center = TileIndex::new(cx, cy);

            // Verify center tile is dry land
            if let Some(tile) = world.map.get(center) {
                if tile.is_water() {
                    continue;
                }
            } else {
                continue;
            }

            // Check distance to already placed towns to prevent clumping
            let too_close = world.towns.values().any(|t| {
                let dx = (t.center.x as i32 - cx as i32).abs();
                let dy = (t.center.y as i32 - cy as i32).abs();
                dx + dy < 12
            });
            if too_close {
                continue;
            }

            let town_id = TownID(town_id_counter);
            town_id_counter += 1;

            let name = town_names[(placed_towns as usize) % town_names.len()].to_string();
            let mut town = Town::new(town_id, name, center);

            // Spawn town buildings in a small radius around the center
            let building_offsets: &[(i16, i16)] = &[
                (0, 0),
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ];

            let mut town_pop = 0;
            for &(ox, oy) in building_offsets {
                let bx = (cx as i16 + ox) as u16;
                let by = (cy as i16 + oy) as u16;
                let b_tile = TileIndex::new(bx, by);

                if let Some(t) = world.map.get(b_tile) {
                    if t.is_water() {
                        continue;
                    }
                } else {
                    continue;
                }

                // Place house tile
                if let Some(tile) = world.map.get_mut(b_tile) {
                    tile.base.kind = TileKind::House;
                }

                let b_id = BuildingID(building_id_counter);
                building_id_counter += 1;

                let kind = if (ox + oy).abs() % 2 == 0 {
                    BuildingKind::Residential
                } else {
                    BuildingKind::Commercial
                };
                let capacity = 20;
                let building = Building::new(b_id, town_id, b_tile, kind, capacity);
                world.buildings.insert(b_id, building);
                town.building_ids.push(b_id);
                town_pop += capacity as u32;
            }

            town.census_population = town_pop;
            world.towns.insert(town_id, town);
            placed_towns += 1;
        }

        // 4. Initialize Company
        let company_id = CompanyID(1);
        let player_company = Company::new(
            company_id,
            "Trans-Atlantic Transport".to_string(),
            config.starting_capital,
            0x00FF8800, // Orange
        );
        world.companies.insert(company_id, player_company);
        world.current_company_id = company_id;

        world
    }
}
