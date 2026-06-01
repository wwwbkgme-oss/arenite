// arenite-world — procedural world generation
//
// Combines approaches from:
//  - terra-awg (C++) : seamless noise sampling, surface level tracking,
//                       biome maps, structured placement (pyramids, icehouses…)
//  - FallingSandEngine: Minecraft-style biome/feature/structure pipeline,
//                       cave generation via domain-warped noise
//
// Pipeline
// ========
// 1. Seed & noise initialisation  (NoiseField)
// 2. Biome map generation          (BiomeMap)
// 3. Surface heightmap             (HeightMap)
// 4. Base fill pass                (WorldBuilder → SimWorld)
// 5. Cave carving                  (CaveCarver)
// 6. Feature/structure placement   (FeaturePlacer)
// 7. Surface decoration            (Decorator)

pub mod biome;
pub mod cave;
pub mod feature;
pub mod height;
pub mod noise_field;
pub mod structure;
pub mod worldgen;

pub use biome::{Biome, BiomeId, BiomeMap, BIOMES};
pub use noise_field::NoiseField;
pub use worldgen::WorldGenerator;

// World depth constants (matching terra-awg proportions):
pub const SURFACE_DEPTH: f64 = 0.18; // fraction of world height
pub const UNDERGROUND_DEPTH: f64 = 0.40;
pub const CAVERN_DEPTH: f64 = 0.65;
pub const UNDERWORLD_START: i32 = 200; // tiles from bottom
