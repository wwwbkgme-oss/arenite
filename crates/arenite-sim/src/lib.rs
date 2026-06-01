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
pub mod physics_type;
pub mod simulator;
pub mod world;
pub mod particle;
pub mod dirty_rect;

pub use chunk::{Chunk, ChunkData};
pub use material::{Material, MaterialInstance, MaterialRegistry};
pub use physics_type::PhysicsType;
pub use simulator::Simulator;
pub use world::SimWorld;
pub use particle::Particle;

// Re-export core position types used heavily throughout sim
pub use arenite_core::pos::{ChunkPos, TilePos, CHUNK_SIZE, CHUNK_AREA};
pub use arenite_core::Color;
