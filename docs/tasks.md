# Arenite Engine — Production Task List

Status: `[x]` done · `[ ]` open · `[~]` partial

---

## Phase 1 — Build Fixes ✅ Complete (T-001..T-007)

- [x] T-001 `rand` dep in `arenite-core/Cargo.toml`
- [x] T-002 `crossbeam-channel` dep in `arenite-physics/Cargo.toml`
- [x] T-003 rapier2d 0.21 — replace `ChannelEventCollector` with `&()` event handler
- [x] T-004 `pollster` / `fastrand` / `glam` deps in `arenite-game/Cargo.toml`
- [x] T-005 `fastrand` dep in `arenite-server/Cargo.toml`
- [x] T-006 Rewrite broken TCP split + fake-reconnect in `arenite-net/server.rs`
- [x] T-007 `cargo check --workspace` → 0 errors

---

## Phase 2 — Correctness & Clippy ✅ Complete (T-008..T-014)

- [x] T-008 Replace hot-path `unwrap()` with safe alternatives in lib crates
- [x] T-009 `tick_one_chunk` returns `Vec<Particle>`; merged in `tick_simulation`
- [x] T-010 Reset `DirtyRect` after GPU upload in `sync_chunks`
- [x] T-011 Per-chunk vertex buffer collected before render pass (not inside)
- [x] T-012 Remove dead imports across workspace
- [x] T-013 `PhysicsPipeline::step` uses `&()` — no orphan event-handler variable
- [x] T-014 `cargo clippy --workspace -D warnings` → 0 errors

---

## Phase 3 — Gray Window Fix 🔴 Critical (T-015..T-019)

- [ ] T-015 **Camera init on frame 1** — `resumed()` must set `camera.position = player.pos`
- [ ] T-016 **Vertex buffer lifetime** — collect `Vec<(Buffer, &ChunkTexture)>` *before*
  `begin_render_pass`; iterate inside the pass
- [ ] T-017 **Dev world size** — default `600×200` tiles; add `WorldGenConfig::dev()`
- [ ] T-018 **Background world gen** — world gen in rayon thread; `WorldState` enum guards
  the game loop until world is ready
- [ ] T-019 **Correct player spawn** — scan world-centre column for first solid tile; place
  player 2 px above surface instead of using hardcoded `height * 0.35`

---

## Phase 4 — Core Systems (T-020..T-024)

- [ ] T-020 `arenite-core/src/error.rs` — `AreniteError` enum + `AreniteResult<T>`
- [ ] T-021 Quad-phase scheduler in `tick_simulation` (4 colour-class phases)
- [ ] T-022 `save_world` / `load_world` — bincode per-chunk to `saves/<name>/`
- [ ] T-023 Server streams `ChunkData` messages to newly joined clients
- [ ] T-024 `S` = save, `L` = load, `P` = pause/unpause

---

## Phase 5 — Gameplay & Rendering (T-025..T-032)

- [ ] T-025 Sky gradient — fullscreen quad behind chunks; top colour from biome `sky_color`
- [ ] T-026 Lighting — propagate per tick; second RGB texture per chunk; multiply in shader
- [ ] T-027 Particle rendering — point-sprite instance buffer rebuilt each frame
- [ ] T-028 HUD — material name, cursor tile coords, FPS/TPS counters, window title
- [ ] T-029 Hotbar — 8 material slots, 1–8 / mouse-wheel to cycle
- [ ] T-030 Pause menu — Esc opens overlay: Resume / Save / Quit
- [ ] T-031 Brush radius — `[` / `]` keys to adjust; display in HUD
- [ ] T-032 Parallax background layer — distant hills/clouds second render pass

---

## Phase 6 — Physics & Simulation (T-033..T-037)

- [ ] T-033 Water pressure — BFS flood-fill upward to `max_pressure` depth
- [ ] T-034 Lava–water interaction — contact spawns `steam` + `stone`
- [ ] T-035 Fire propagation — `wood` / `grass` flammable; burn → `smoke`
- [ ] T-036 Wind — per-biome horizontal drift applied to `Gas` pixels
- [ ] T-037 Rigidbody demo — falling crate on spawn using `RigidBody::pre_sim_stamp`

---

## Phase 7 — Developer Experience (T-038..T-044)

- [ ] T-038 Unit tests: `sim_sand_falls`, `sim_liquid_spreads`, `worldgen_smoke`,
  `chunk_pos_roundtrip`, `save_load_roundtrip`
- [ ] T-039 Integration test: generate small world, tick 100×, assert active pixels > 0
- [ ] T-040 `benches/sim_bench.rs` — criterion: `tick_64_chunks` baseline < 4 ms
- [ ] T-041 `CHANGELOG.md` (Keep-a-Changelog)
- [ ] T-042 `CONTRIBUTING.md` — build, style, PR checklist
- [ ] T-043 `arenite.toml.example` + `arenite-server.toml.example`
- [ ] T-044 `docs/architecture.md` — crate dep graph + data-flow

---

## Phase 8 — Performance (T-045..T-049)

- [ ] T-045 Pre-allocate particles with capacity 4096
- [ ] T-046 Texture atlas — pack ≤512 chunks into 2048² GPU texture; UBO of UV offsets
- [ ] T-047 Chunk eviction — unload chunks > `UNLOAD_RADIUS = 12` from player
- [ ] T-048 Skip sim for `dirty.max_dynamic == 0` chunks
- [ ] T-049 `rayon::par_iter` within each quad phase once non-overlapping is proven

---

## Phase 9 — Release Infrastructure (T-050..T-057)

- [ ] T-050 CI: `cargo audit` security step
- [ ] T-051 CI: `cargo deny` license + duplicate-dep check
- [ ] T-052 CI: Windows build target (`windows-latest`)
- [ ] T-053 GitHub Release workflow — binaries for `linux-x64`, `macos-arm64`, `windows-x64`
- [ ] T-054 Publish `arenite-core`, `arenite-sim`, `arenite-world` to crates.io
- [ ] T-055 Full `cargo check` + `cargo test` green in CI
- [ ] T-056 Repo topics: `rust` `game-engine` `cellular-automata` `falling-sand`
  `procedural-generation` `wgpu` `terraria-like`
- [ ] T-057 README screenshots / GIF section with gameplay footage placeholder
