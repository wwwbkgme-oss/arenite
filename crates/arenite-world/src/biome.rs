use arenite_core::id::Id;
use arenite_core::Color;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;

/// Unique biome type identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BiomeId(pub u8);

impl BiomeId {
    pub const PLAINS:    Self = BiomeId(0);
    pub const DESERT:    Self = BiomeId(1);
    pub const TUNDRA:    Self = BiomeId(2);
    pub const JUNGLE:    Self = BiomeId(3);
    pub const OCEAN:     Self = BiomeId(4);
    pub const CAVERN:    Self = BiomeId(5);
    pub const UNDERWORLD:Self = BiomeId(6);
}

/// Static description of a biome's surface materials and visual parameters.
#[derive(Clone, Debug)]
pub struct Biome {
    pub id:            BiomeId,
    pub name:          &'static str,
    /// Material placed in the top 1-3 tiles of the surface.
    pub surface_mat:   &'static str,  // material registry key
    /// Material placed in the subsurface (4-20 tiles deep).
    pub subsurface_mat:&'static str,
    /// Background fill below subsurface.
    pub fill_mat:      &'static str,
    /// Sky/background colour at this biome's horizon.
    pub sky_color:     Color,
    /// Surface height modifier: positive = higher terrain.
    pub height_mod:    f64,
    /// Cave frequency multiplier (> 1 = more caves).
    pub cave_factor:   f64,
}

pub const BIOMES: &[Biome] = &[
    Biome {
        id: BiomeId::PLAINS,
        name: "Plains",
        surface_mat:    "grass",
        subsurface_mat: "dirt",
        fill_mat:       "stone",
        sky_color:       Color::rgb(135, 206, 235),
        height_mod:     0.0,
        cave_factor:    1.0,
    },
    Biome {
        id: BiomeId::DESERT,
        name: "Desert",
        surface_mat:    "sand",
        subsurface_mat: "sand",
        fill_mat:       "stone",
        sky_color:       Color::rgb(210, 190, 140),
        height_mod:    -0.05,
        cave_factor:    0.8,
    },
    Biome {
        id: BiomeId::TUNDRA,
        name: "Tundra",
        surface_mat:    "snow",
        subsurface_mat: "dirt",
        fill_mat:       "stone",
        sky_color:       Color::rgb(180, 210, 240),
        height_mod:     0.02,
        cave_factor:    0.9,
    },
    Biome {
        id: BiomeId::JUNGLE,
        name: "Jungle",
        surface_mat:    "grass",
        subsurface_mat: "dirt",
        fill_mat:       "stone",
        sky_color:       Color::rgb(80, 160, 60),
        height_mod:     0.08,
        cave_factor:    1.3,
    },
    Biome {
        id: BiomeId::OCEAN,
        name: "Ocean",
        surface_mat:    "sand",
        subsurface_mat: "sand",
        fill_mat:       "stone",
        sky_color:       Color::rgb(30, 100, 180),
        height_mod:    -0.20,
        cave_factor:    1.0,
    },
    Biome {
        id: BiomeId::CAVERN,
        name: "Cavern",
        surface_mat:    "stone",
        subsurface_mat: "stone",
        fill_mat:       "stone",
        sky_color:       Color::rgb(20, 20, 30),
        height_mod:     0.0,
        cave_factor:    2.0,
    },
    Biome {
        id: BiomeId::UNDERWORLD,
        name: "Underworld",
        surface_mat:    "stone",
        subsurface_mat: "stone",
        fill_mat:       "stone",
        sky_color:       Color::rgb(60, 10, 5),
        height_mod:     0.0,
        cave_factor:    1.5,
    },
];

/// A 1D horizontal biome assignment map.
/// Each entry gives the biome for one tile column.
pub struct BiomeMap {
    /// BiomeId per world-x tile column.
    pub columns: Vec<BiomeId>,
}

impl BiomeMap {
    pub fn new(width: i32, noise_fn: &crate::noise_field::NoiseField) -> Self {
        let mut columns = Vec::with_capacity(width as usize);
        for x in 0..width {
            let nx = x as f64 / width as f64 * 4.0; // 4 biome "periods" per world
            let t = noise_fn.biome_selector(nx);
            let biome = Self::select_biome(t, x, width);
            columns.push(biome);
        }
        Self { columns }
    }

    /// Select a biome based on a 0..1 selector value and column.
    fn select_biome(t: f64, x: i32, width: i32) -> BiomeId {
        // Oceans are always at the edges.
        let edge_frac = (x as f64 / width as f64).min(1.0 - x as f64 / width as f64);
        if edge_frac < 0.08 {
            return BiomeId::OCEAN;
        }
        // Interior biomes.
        match (t * 4.0) as u8 % 4 {
            0 => BiomeId::PLAINS,
            1 => BiomeId::DESERT,
            2 => BiomeId::TUNDRA,
            _ => BiomeId::JUNGLE,
        }
    }

    pub fn get(&self, x: i32) -> BiomeId {
        let w = self.columns.len() as i32;
        let idx = x.rem_euclid(w) as usize;
        self.columns[idx]
    }

    pub fn get_biome_data(id: BiomeId) -> &'static Biome {
        BIOMES.iter().find(|b| b.id == id).unwrap_or(&BIOMES[0])
    }
}
