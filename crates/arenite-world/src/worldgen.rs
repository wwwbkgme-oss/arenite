use arenite_core::pos::{ChunkPos, CHUNK_SIZE};
use arenite_core::rng::AreniteRng;
use arenite_sim::SimWorld;
use arenite_sim::chunk::ChunkData;
use arenite_sim::material::{MaterialInstance, flags};
use arenite_sim::physics_type::PhysicsType;
use crate::biome::{Biome, BiomeId, BiomeMap};
use crate::cave::{CaveCarver, CaveContext};
use crate::feature::FeaturePlacer;
use crate::height::HeightMap;
use crate::noise_field::NoiseField;
use crate::structure;

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
            width:        4200,
            height:       1200,
            seed:         12345,
            sea_level:    600,
            cave_density: 1.0,
        }
    }
}

impl WorldGenConfig {
    pub fn small() -> Self {
        Self { width: 2100, height: 600, ..Default::default() }
    }

    /// Tiny world for tests and quick iteration.
    pub fn dev() -> Self {
        Self { width: 600, height: 200, seed: fastrand::u64(..), ..Default::default() }
    }
}

/// Drives the full world generation pipeline.
///
/// Pipeline (inspired by terra-awg, FallingSandEngine, Terraria, and Starbound):
/// 1. Noise field + biome map
/// 2. Surface heightmap (torus-sampled for seamless wrapping)
/// 3. Base fill pass (biome-aware surface, subsurface, stone layers)
/// 4. Cave carving (domain-warped double-FBM)
/// 5. Ore placement (copper → iron → gold → titanium → diamond)
/// 6. Clay pockets near surface
/// 7. Surface decorations (grass tufts, flowers)
/// 8. Tree placement (biome-appropriate species)
/// 9. Floating sky islands
/// 10. Cave mushrooms + crystal clusters
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

        // Per-column biome ID array (used by structure placement for biome-correct trees).
        let biome_ids: Vec<u8> = (0..cfg.width)
            .map(|x| biomes.get(x).0)
            .collect();

        // Collect per-column height modifiers from biome data.
        let height_mods: Vec<f64> = (0..cfg.width)
            .map(|x| BiomeMap::get_biome_data(biomes.get(x)).height_mod)
            .collect();

        let heights = HeightMap::generate(
            cfg.width, cfg.height, &noise, &height_mods,
        );

        log::info!("Surface y range: {}..{}", heights.min_surface(), heights.max_surface());

        // ── Base fill ──────────────────────────────────────────────────────
        let mut world = SimWorld::new(cfg.seed);
        self.fill_world(&mut world, &noise, &biomes, &heights);

        // ── Cave carving ───────────────────────────────────────────────────
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

        let mut rng = AreniteRng::from_seed(cfg.seed ^ 0xabcdef);

        // ── Ore placement (expanded: copper/iron/gold/titanium/diamond) ────
        log::info!("Placing ores…");
        FeaturePlacer::place_ores(
            &mut world,
            cfg.width,
            cfg.height,
            &noise,
            heights.underground,
            heights.cavern,
        );

        // ── Clay pockets near surface ──────────────────────────────────────
        FeaturePlacer::place_clay(&mut world, cfg.width, &heights.surface, &mut rng);

        // ── Surface decorations (grass tufts, flowers) ────────────────────
        FeaturePlacer::place_surface_decorations(
            &mut world, cfg.width, &heights.surface, &mut rng,
        );

        // ── Surface structures and biome-aware trees ──────────────────────
        log::info!("Placing structures and trees…");
        structure::place_structures(
            &mut world,
            cfg.width,
            &heights.surface,
            &mut rng,
            &biome_ids,
        );

        // ── Floating sky islands (Terraria sky islands / Starbound sky biome)
        log::info!("Placing floating islands…");
        structure::place_floating_islands(
            &mut world,
            cfg.width,
            cfg.height,
            &mut rng,
        );

        // ── Cave mushrooms (Terraria mushroom biome) ──────────────────────
        structure::place_mushrooms(
            &mut world,
            cfg.width,
            heights.underground,
            heights.cavern,
            &mut rng,
        );

        // ── Crystal clusters (Starbound crystal caves) ────────────────────
        structure::place_crystal_clusters(
            &mut world,
            cfg.width,
            heights.cavern,
            cfg.height,
            &mut rng,
        );

        log::info!(
            "World generation complete. {} chunks loaded.",
            world.loaded_chunk_count()
        );
        world
    }

    // ── Fill pass ────────────────────────────────────────────────────────────

    /// Fill base terrain: solid below surface, air above.
    fn fill_world(
        &self,
        world:   &mut SimWorld,
        noise:   &NoiseField,
        biomes:  &BiomeMap,
        heights: &HeightMap,
    ) {
        let cfg = &self.config;

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

        // Above the surface: air (or water if inside ocean biome below sea level).
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

        let depth = wy - surface_y;

        // ── Surface layer (top 1-3 tiles) ──────────────────────────────────
        if depth < 3 {
            return self.surface_pixel(biome);
        }

        // ── Subsurface (3–20 tiles deep) — biome-specific ─────────────────
        if depth < 20 {
            return self.subsurface_pixel(biome);
        }

        // ── Underworld ────────────────────────────────────────────────────
        if wy > heights.underworld {
            return MaterialInstance {
                id:      0,
                physics: PhysicsType::Solid,
                color:   arenite_core::Color::rgb(40, 20, 30),
                light:   [0.02, 0.0, 0.0],  // faint ember glow
                data:    0,
            };
        }

        // ── Stone fill — slight noise variation for natural look ──────────
        let nx  = wx as f64 / cfg.width  as f64 * 20.0;
        let ny  = wy as f64 / cfg.height as f64 * 20.0;
        let jit = (noise.feature(nx, ny) * 15.0) as i16;
        let v   = (128_i16 + jit).clamp(80, 180) as u8;

        MaterialInstance {
            id:      0,
            physics: PhysicsType::Solid,
            color:   arenite_core::Color::rgb(v, v, v),
            light:   [0.0; 3],
            data:    0,
        }
    }

    /// Return the surface pixel for a given biome.
    /// Tundra gets Ice (Sand physics, slides); Jungle gets grass but
    /// with flammable flag.  Desert/Ocean get sand.
    fn surface_pixel(&self, biome: &Biome) -> MaterialInstance {
        match biome.id {
            BiomeId::TUNDRA => MaterialInstance {
                id:      0,
                physics: PhysicsType::Sand,  // Ice slides like sand
                color:   arenite_core::Color::ICE,
                light:   [0.0; 3],
                data:    0,
            },
            BiomeId::DESERT | BiomeId::OCEAN => MaterialInstance {
                id:      0,
                physics: PhysicsType::Sand,
                color:   arenite_core::Color::SAND,
                light:   [0.0; 3],
                data:    0,
            },
            BiomeId::PLAINS | BiomeId::JUNGLE => MaterialInstance {
                id:      0,
                physics: PhysicsType::Solid,
                color:   arenite_core::Color::GRASS,
                light:   [0.0; 3],
                data:    flags::FLAMMABLE,  // grass burns (T-035)
            },
            _ => MaterialInstance {
                id:      0,
                physics: PhysicsType::Solid,
                color:   arenite_core::Color::STONE,
                light:   [0.0; 3],
                data:    0,
            },
        }
    }

    /// Return the subsurface pixel for a given biome.
    /// Jungle gets Mud (dark, rich soil).  Tundra gets frozen dirt.
    /// Others get standard Dirt.
    fn subsurface_pixel(&self, biome: &Biome) -> MaterialInstance {
        let (color, data) = match biome.id {
            BiomeId::JUNGLE => (arenite_core::Color::MUD, 0u16),
            BiomeId::TUNDRA => (arenite_core::Color::rgb(110, 100, 130), 0), // frozen soil
            _               => (arenite_core::Color::DIRT, 0),
        };
        MaterialInstance {
            id:      0,
            physics: PhysicsType::Solid,
            color,
            light:   [0.0; 3],
            data,
        }
    }
}
