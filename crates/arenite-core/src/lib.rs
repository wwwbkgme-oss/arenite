// arenite-core — shared primitives for the Arenite Engine
//
// Provides:
//  - Math types (re-exported from `glam`)
//  - Generic `Registry<T>` + `RegistryId<T>` system
//  - `Color` (RGBA u8)
//  - `TilePos`, `ChunkPos`, `WorldPos` coordinate types
//  - Error types

pub mod color;
pub mod id;
pub mod math;
pub mod pos;
pub mod registry;
pub mod rng;

pub use color::Color;
pub use id::Id;
pub use pos::{ChunkPos, TilePos, WorldPos};
pub use registry::{Registry, RegistryId};

/// Re-export glam types so downstream crates share the same version.
pub use glam::{IVec2, IVec3, UVec2, Vec2, Vec3, Vec4};
