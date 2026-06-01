use arenite_core::pos::{ChunkPos, CHUNK_SIZE};
use arenite_core::rng::AreniteRng;
use arenite_sim::SimWorld;
use arenite_sim::chunk::ChunkData;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;
use crate::biome::{Biome, BiomeId, BiomeMap};
use crate::cave::{CaveCarver, CaveContext};
use crate::feature::FeaturePlacer;
use crate::height::HeightMap;
use crate::noise_field::NoiseField;

/// Configuration for world generation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldGenConfig {
    pub width:        i32,
    pub height:       i32,
    pub seed:         u64,
    pub sea_level:    i32,
    /// Multiplier applied to cave carving thresholds.
    pub cave_density: f64,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            width:       4200,
            height:      1200,
            seed:        12345,
            sea_level:   600,
            cave_density:1.0,
        }
    }
}

impl WorldGenConfig {
    pub fn small() -> Self {
        Self { width: 2100, height: 600, ..Default::default() }
    }
}

/// Drives the full world generation pipeline.
pub struct WorldGenerator {
    pub config: WorldGenConfig,
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        Self { config }
    }

    /// Generate a complete world into a new `SimWorld`.
    pub fn generate(&self) -> SimWorld {
        let cfg = &self.config;
        log::info!(
            "Generating world {}×{} seed={}",
            cfg.width, cfg.height, cfg.seed
        );

        let noise  = NoiseField::new(cfg.seed);
        let biomes = BiomeMap::new(cfg.width, &noise);

        // Collect per-column height modifiers from biome data.
        let height_mods: Vec<f64> = (0..cfg.width)
            .map(|x| BiomeMap::get_biome_data(biomes.get(x)).height_mod)
            .collect();

        let heights = HeightMap::generate(
            cfg.width, cfg.height, &noise, &height_mods,
        );

        log::info!("Surface y range: {}..{}", heights.min_surface(), heights.max_surface());

        // Create the SimWorld and fill all chunks.
        let mut world = SimWorld::new(cfg.seed);
        self.fill_world(&mut world, &noise, &biomes, &heights);

        // Carve caves.
        let carver = CaveCarver::default();
        log::info!("Carving caves…");
        let cave_ctx = CaveContext {
            world_width:   cfg.width,
            world_height:  cfg.height,
            noise:         &noise,
            underground_y: heights.underground,
            cavern_y:      heights.cavern,
            cave_factor:   cfg.cave_density,
        };
        carver.carve_region(&mut world, 0, cfg.width, &heights.surface, &cave_ctx);

        // Place ores.
        log::info!("Placing ores…");
        FeaturePlacer::place_ores(
            &mut world,
            cfg.width,
            cfg.height,
            &noise,
            heights.underground,
            heights.cavern,
        );

        // Surface decorations.
        let mut rng = AreniteRng::from_seed(cfg.seed ^ 0xabcdef);
        FeaturePlacer::place_surface_decorations(
            &mut world, cfg.width, &heights.surface, &mut rng,
        );

        log::info!(
            "World generation complete. {} chunks loaded.",
            world.loaded_chunk_count()
        );
        world
    }

    /// Fill base terrain: solid below surface, air above.
    /// Also sets sea level water fill.
    fn fill_world(
        &self,
        world:   &mut SimWorld,
        noise:   &NoiseField,
        biomes:  &BiomeMap,
        heights: &HeightMap,
    ) {
        let cfg = &self.config;

        // Pre-generate all chunks in parallel, then insert.
        let chunks_x = (cfg.width  as f64 / CHUNK_SIZE as f64).ceil() as i32;
        let chunks_y = (cfg.height as f64 / CHUNK_SIZE as f64).ceil() as i32;

        for cy in 0..chunks_y {
            for cx in 0..chunks_x {
                let cp = ChunkPos::new(cx, cy);
                let data = self.fill_chunk(cp, noise, biomes, heights);
                world.insert_chunk(cp, data);
            }
        }
    }

    fn fill_chunk(
        &self,
        cp:      ChunkPos,
        noise:   &NoiseField,
        biomes:  &BiomeMap,
        heights: &HeightMap,
    ) -> ChunkData {
        let cfg = &self.config;
        let mut data = ChunkData::new_empty();
        let ox = cp.x * CHUNK_SIZE;
        let oy = cp.y * CHUNK_SIZE;

        for ly in 0..CHUNK_SIZE {
            let wy = oy + ly;
            for lx in 0..CHUNK_SIZE {
                let wx = ox + lx;
                if wx >= cfg.width || wy >= cfg.height { continue; }

                let biome_id = biomes.get(wx);
                let biome    = BiomeMap::get_biome_data(biome_id);
                let sy       = heights.surface_at(wx);

                let mat = self.pixel_at(wx, wy, sy, biome, heights, noise);
                data.set(lx, ly, mat);
            }
        }
        data
    }

    fn pixel_at(
        &self,
        wx: i32, wy: i32,
        surface_y: i32,
        biome: &Biome,
        heights: &HeightMap,
        noise: &NoiseField,
    ) -> MaterialInstance {
        let cfg = &self.config;

        // Above the surface: air (or water if below sea level).
        if wy < surface_y {
            if wy >= cfg.sea_level && biome.id == BiomeId::OCEAN {
                return MaterialInstance {
                    id:      0,
                    physics: PhysicsType::Liquid,
                    color:   arenite_core::Color::WATER,
                    light:   [0.0; 3],
                    data:    0,
                };
            }
            return MaterialInstance::air();
        }

        // Surface layer (top 3 tiles).
        let depth = wy - surface_y;
        let (physics, color) = if depth < 3 {
            (PhysicsType::Solid, Self::biome_surface_color(biome))
        } else if depth < 20 {
            (PhysicsType::Solid, arenite_core::Color::DIRT)
        } else if wy > heights.underworld {
            // Underworld: obsidian-like dark stone.
            (PhysicsType::Solid, arenite_core::Color::rgb(40, 20, 30))
        } else {
            // Stone below.
            let nx = wx as f64 / cfg.width  as f64 * 20.0;
            let ny = wy as f64 / cfg.height as f64 * 20.0;
            let jit = (noise.feature(nx, ny) * 15.0) as i16;
            let base = 128_i16;
            let v = (base + jit).clamp(80, 180) as u8;
            (PhysicsType::Solid, arenite_core::Color::rgb(v, v, v))
        };

        MaterialInstance {
            id: 0,
            physics,
            color,
            light: [0.0; 3],
            data: 0,
        }
    }

    fn biome_surface_color(biome: &Biome) -> arenite_core::Color {
        match biome.id {
            BiomeId::PLAINS  | BiomeId::JUNGLE => arenite_core::Color::GRASS,
            BiomeId::DESERT  | BiomeId::OCEAN  => arenite_core::Color::SAND,
            BiomeId::TUNDRA                     => arenite_core::Color::SNOW,
            _                                   => arenite_core::Color::STONE,
        }
    }
}
