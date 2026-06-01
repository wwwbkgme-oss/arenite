# Changelog

All notable changes to Arenite Engine follow [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

### Added
- `arenite-core`: `AreniteError` / `AreniteResult` error types
- `arenite-sim`: world save/load via bincode (`S` / `L` keys)
- `arenite-sim`: quad-phase tick scheduler (4 colour-class phases)
- `arenite-game`: background world generation (event loop stays responsive)
- `arenite-game`: `find_spawn()` — scans centre column for actual surface
- `arenite-game`: brush size keys `[` / `]`; 8 material slots
- `arenite-game`: window title with fps / material / brush / coords
- Unit tests: `pos_rng`, `sand_falls`, `sand_rests`, `water_falls`,
  `save_load_roundtrip`

### Fixed
- **Gray window**: camera was at `Vec2::ZERO` on frame 1 (T-015)
- **Vertex buffer lifetime**: temp buffers created inside render pass caused
  silent no-draw; replaced with persistent per-batch buffer (T-016)
- `cargo check --workspace` was failing (missing deps: rand, crossbeam-channel,
  pollster, fastrand, glam; wgpu 22.1 API mismatches) — all resolved (T-001..T-007)
- `cargo clippy --workspace -D warnings` — 0 errors (T-014)
- Rapier2d 0.21: `ChannelEventCollector` removed; `&()` handler used (T-003)
- Broken TCP split code in `arenite-net/server.rs` (T-006)

## [0.1.0] — 2025-06-01

### Added
- Initial workspace with 8 crates + 2 binaries
- `arenite-core`: Color, Id<T>, Registry<T>, TilePos/ChunkPos/WorldPos, AreniteRng
- `arenite-sim`: PhysicsType, 15 materials, ChunkData/DirtyRect, Particle,
  SimContext (3×3 neighbourhood), Simulator (sand/liquid/gas/fire CA), SimWorld
- `arenite-world`: NoiseField (FBM OpenSimplex + domain-warp caves), BiomeMap,
  HeightMap (toroidal 4D), CaveCarver, FeaturePlacer (ores), Structure stamps,
  WorldGenerator
- `arenite-physics`: PhysicsWorld (Rapier2d), RigidBody sand bridge
- `arenite-render`: Camera2D, Vertex2D, ChunkTexture, LightPropagator,
  WorldPipeline (WGSL), AreniteRenderer
- `arenite-net`: NetMessage, bincode codec, GameServer (tokio), GameClient
- `arenite-game`: winit event loop, player AABB physics, pixel painting
- `arenite-server`: headless 20 TPS server + network
