# Arenite Engine — Production Task List

Status legend: `[ ]` open · `[x]` done · `[~]` partial

---

## Phase 1 — Build Fixes (Blocker)

- [ ] **T-001** Add missing `rand` dep to `arenite-core/Cargo.toml`
- [ ] **T-002** Add missing `crossbeam-channel` dep to `arenite-physics/Cargo.toml`
- [ ] **T-003** Fix rapier2d 0.21 API: `ChannelEventCollector` constructor changed; pass `()` event handler instead
- [ ] **T-004** Add missing `pollster` dep to `arenite-game/Cargo.toml`
- [ ] **T-005** Add missing `fastrand` dep to `arenite-server/Cargo.toml`
- [ ] **T-006** Fix broken `OwnedReadHalf::from(...)` + `TcpStream::from_std(connect(...))` in `arenite-net/src/server.rs`
- [ ] **T-007** `cargo check --workspace` → zero errors

## Phase 2 — Correctness & Safety

- [ ] **T-008** Replace `unwrap()` / `expect()` in library crates with `Result` propagation or safe defaults
- [ ] **T-009** Fix `SimWorld::tick_one_chunk` — merge `particles_local` back into `self.particles` after each chunk tick
- [ ] **T-010** Reset `ChunkData::dirty` after GPU texture upload in `AreniteRenderer::sync_chunks`
- [ ] **T-011** Implement per-chunk quad offset in renderer via per-frame vertex re-upload (removes the broken shared quad buffer)
- [ ] **T-012** Fix `arenite-world/src/biome.rs` — remove unused `use arenite_sim::material::MaterialInstance` import
- [ ] **T-013** Fix `PhysicsWorld::step` — remove orphaned `event_handler` variable; pass `&()` directly to pipeline
- [ ] **T-014** `cargo clippy --workspace -- -D warnings` → zero warnings

## Phase 3 — Missing Core Systems

- [ ] **T-015** Implement proper rayon quad-scheduler in `SimWorld::tick_simulation` (2×2 independent chunk quads in parallel)
- [ ] **T-016** Add `ChunkStore` trait: `load_chunk_on_demand` when player approaches unloaded region
- [ ] **T-017** World save / load: `bincode` serialize `SimWorld` chunks to `saves/<name>/chunks/` directory
- [ ] **T-018** Multiplayer chunk streaming: server sends `NetMessage::ChunkData` on player join and chunk enter
- [ ] **T-019** Add `crates/arenite-core/src/error.rs`: `AreniteError` enum + `AreniteResult<T>` alias

## Phase 4 — Gameplay Completeness

- [ ] **T-020** HUD: selected material label, tile coordinates under cursor, FPS/TPS counters (wgpu text or egui)
- [ ] **T-021** Particle rendering: upload particle positions as point-sprite instance buffer each frame
- [ ] **T-022** Background sky gradient: full-screen quad behind world, interpolates sky colour per biome
- [ ] **T-023** Inventory: 10-slot hotbar; pick up pixels into stack, place from stack
- [ ] **T-024** Lighting integration: call `LightPropagator::propagate_chunk` after sim tick; upload light map as second texture
- [ ] **T-025** Water physics improvement: pressure propagation (fill from bottom), evaporation near lava

## Phase 5 — Developer Experience

- [ ] **T-026** Add `tests/` directory with unit tests: `sim_sand_falls`, `sim_liquid_spreads`, `worldgen_smoke`, `chunk_pos_roundtrip`
- [ ] **T-027** Add `benches/sim_bench.rs` with criterion: `bench_tick_1000_chunks`, `bench_worldgen_small`
- [ ] **T-028** Add `CHANGELOG.md` following Keep-a-Changelog format
- [ ] **T-029** Add `CONTRIBUTING.md`: build instructions, code style, PR checklist
- [ ] **T-030** Add example configs: `arenite.toml.example`, `arenite-server.toml.example`
- [ ] **T-031** Add `docs/architecture.md`: crate dependency graph, data-flow diagrams, design rationale

## Phase 6 — Performance

- [ ] **T-032** Profile sim tick with `tracing` spans; identify hot paths
- [ ] **T-033** Pre-allocate particle Vec with capacity 4096 to avoid reallocations
- [ ] **T-034** Texture atlas: pack all chunk textures into one large GPU texture + UV offset UBO (eliminates per-chunk bind group switch)
- [ ] **T-035** Only upload dirty chunks (full dirty rect, not whole texture)
- [ ] **T-036** Background thread for world generation (rayon scope, not blocking the game loop)

## Phase 7 — Release Infrastructure

- [ ] **T-037** CI: add `cargo audit` step (security advisories)
- [ ] **T-038** CI: add `cargo deny` for license and duplicate-dep checks
- [ ] **T-039** CI: add Windows build target (`windows-latest`)
- [ ] **T-040** GitHub Release workflow: build release binaries for linux-x64, macos-arm64, windows-x64 on tag push
- [ ] **T-041** Publish `arenite-core`, `arenite-sim`, `arenite-world` to crates.io (no API-key auth needed for workflow stub)
- [ ] **T-042** Write `README.md` screenshots section (placeholder for gameplay GIFs)
- [ ] **T-043** Set repo topics on GitHub: `rust`, `game-engine`, `cellular-automata`, `procedural-generation`, `wgpu`
- [ ] **T-044** Final `cargo check --workspace` + `cargo test --workspace` → green
