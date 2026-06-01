// arenite-sim — falling-sand cellular automata engine
//
// Implements a multi-threaded pixel simulation where every pixel has a
// PhysicsType that governs how it interacts with neighbours each tick.
//
// Architecture
// ============
// World ──► ChunkStore ──► [Chunk; N]  (each Chunk owns CHUNK_AREA pixels)
//                ▲
//            Simulator  (drives per-tick updates, reads dirty rects)
//
// The simulation subdivides the world into independent 2×2 chunk "quads" and
// runs each quad in parallel on rayon threads.  Pixels that cross chunk
// borders are handled via a 3×3 neighbourhood view (SimContext).

pub mod chunk;
pub mod material;
pub mod particle;
pub mod physics_type;
pub mod save;
pub mod simulator;
pub mod world;

pub use chunk::{Chunk, ChunkData};
pub use material::{make_instance, Material, MaterialInstance, MaterialRegistry};
pub use particle::Particle;
pub use physics_type::PhysicsType;
pub use save::{load_world, save_world};
pub use simulator::Simulator;
pub use world::SimWorld;

// Re-export core position types used heavily throughout sim
pub use arenite_core::pos::{ChunkPos, TilePos, CHUNK_AREA, CHUNK_SIZE};
pub use arenite_core::Color;
