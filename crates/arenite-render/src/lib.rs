// arenite-render — wgpu GPU renderer
//
// Implements the rendering pipeline for Arenite Engine:
//  1. Chunk texture updates (CPU→GPU pixel upload)
//  2. World / chunk mesh rendering (textured quads)
//  3. Terraria-style RGB colour lighting pass (FallingSandEngine pattern)
//  4. Post-processing (vignette, colour grading)
//  5. UI rendering (egui integration stub)
//
// Architecture
// ============
// Renderer owns the wgpu Device/Queue/Surface.
// Each loaded chunk gets a RGBA8 wgpu::Texture that mirrors ChunkData::pixels.
// A simple fullscreen-quad pipeline composites chunk textures with lighting.

pub mod camera;
pub mod chunk_tex;
pub mod lighting;
pub mod pipeline;
pub mod renderer;
pub mod vertex;

pub use camera::Camera2D;
pub use renderer::AreniteRenderer;
