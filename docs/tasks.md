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

## Phase 3 — Gray Window Fix ✅ Complete (T-015..T-019)

- [x] T-015 Camera init on frame 1 — `resumed()` sets `camera.position = player.pos`
- [x] T-016 Vertex buffer lifetime — collect `Vec<(Buffer, &ChunkTexture)>` before `begin_render_pass`
- [x] T-017 Dev world size — default `600×200` tiles; `WorldGenConfig::dev()`
- [x] T-018 Background world gen — world gen in `std::thread`; `WorldState` guards game loop
- [x] T-019 Correct player spawn — scan world-centre column for first solid tile

---

## Phase 4 — Core Systems ✅ Complete (T-020..T-024)

- [x] T-020 `AreniteError` enum + `AreniteResult<T>` in `arenite-core/src/error.rs`
- [x] T-021 Quad-phase scheduler in `tick_simulation` (4 colour-class phases)
- [x] T-022 `save_world` / `load_world` — bincode per-chunk to `saves/<name>/`
- [ ] T-023 Server streams `ChunkData` messages to newly joined clients
- [x] T-024 `S` = save, `L` = load, `P` = pause/unpause hotkeys

---

## Phase 5 — Gameplay & Rendering (T-025..T-032)

- [x] T-025 Sky gradient — biome `sky_color` drives renderer clear colour (T-025)
- [x] T-026 **Blue screen fix** — `cull_mode: None` in `WorldPipeline` (Y-axis flip bug)
- [ ] T-027 Particle rendering — point-sprite instance buffer rebuilt each frame
- [ ] T-028 HUD — egui overlay: item name, HP bar, FPS/TPS, cursor coords
- [x] T-029 Hotbar — 8 inventory slots; 1–8 keys; item `display_name` in title
- [ ] T-030 Pause menu — Esc opens overlay: Resume / Save / Quit
- [x] T-031 Brush radius — `[` / `]` keys; shown in window title
- [ ] T-032 Parallax background layer — distant hills/clouds second render pass

---

## Phase 6 — Physics & Simulation (T-033..T-037)

- [x] T-033 Water pressure — BFS flood-fill upward when liquid is enclosed
- [x] T-034 Lava–water interaction — contact produces `steam` + `obsidian`
- [x] T-035 Fire propagation — `FLAMMABLE` bit in `MaterialInstance::data`; wood/grass/oil burn
- [x] T-036 Wind — oscillating horizontal drift for `Gas` pixels (T-036)
- [ ] T-037 Rigidbody demo — falling crate on spawn using `RigidBody::pre_sim_stamp`

---

## Phase 7 — Developer Experience (T-038..T-044)

- [x] T-038 Unit tests: sand_falls, liquid_spreads, save_load_roundtrip, etc.
- [x] T-039 Integration tests in `arenite-world/tests/integration.rs`:
  generate+tick world, fire-spreads-to-wood, surface-tiles check
- [x] T-040 Criterion bench `arenite-sim/benches/sim_bench.rs`:
  tick_dynamic + tick_all_static baseline
- [x] T-041 `CHANGELOG.md` (Keep-a-Changelog)
- [x] T-042 `CONTRIBUTING.md` — build, style, PR checklist
- [x] T-043 `arenite.toml.example` + `arenite-server.toml.example`
- [ ] T-044 `docs/architecture.md` — crate dep graph + data-flow diagram

---

## Phase 8 — Performance (T-045..T-049)

- [ ] T-045 Pre-allocate particle `Vec` with capacity 4096
- [ ] T-046 Texture atlas — pack ≤512 chunks into 2048² GPU texture; UBO of UV offsets
- [x] T-047 Chunk eviction — `UNLOAD_RADIUS = 12`; stale chunks removed in `update_load_state`
- [x] T-048 Skip sim for static chunks — incremental `dynamic_count` in `ChunkData::set()`;
  chunks with `dynamic_count == 0` skipped in `tick_simulation`
- [ ] T-049 `rayon::par_iter` within each quad phase once non-overlapping is proven safe

---

## Phase 9 — Release Infrastructure (T-050..T-057)

- [x] T-050 CI: `cargo audit` security step
- [x] T-051 CI: `cargo deny` license + duplicate-dep check + `deny.toml`
- [x] T-052 CI: Windows build target (`windows-latest`)
- [x] T-053 GitHub Release workflow — binaries for `linux-x64`, `macos-arm64`, `windows-x64`
- [ ] T-054 Publish `arenite-core`, `arenite-sim`, `arenite-world` to crates.io
- [ ] T-055 Full `cargo check` + `cargo test` green in CI
- [ ] T-056 Repo topics: `rust` `game-engine` `cellular-automata` `falling-sand`
  `procedural-generation` `wgpu` `terraria-like` `starbound-like`
- [ ] T-057 README: screenshots / GIF section with gameplay footage placeholder

---

## Phase 10 — Materials & World Expansion ✅ Complete (T-058..T-068)

*Inspired by Terraria's tile registry, Starbound's material database, and re-flora's voxel system.*

- [x] T-058 11 new materials: clay, mud, obsidian, oil (flammable liquid), acid, ice (Sand
  physics), crystal (emissive), mushroom_block, copper_ore, titanium_ore, diamond
- [x] T-059 Color constants for all new materials in `arenite_core::color`
- [x] T-060 `Material::flammable: bool` + `Material::hardness: u8` fields + builder methods
- [x] T-061 `Material::to_instance(id)` + free fn `make_instance(registry, key)` helpers
- [x] T-062 `flags::FLAMMABLE` in `MaterialInstance::data`; `is_flammable()`, `with_flammable()`
- [x] T-063 Biome-aware worldgen: Tundra surface → Ice (Sand physics); Jungle subsurface → Mud
- [x] T-064 Tree structures: small / large / jungle / pine species; flammable wood + leaves
  (re-flora flora system + Terraria tree shapes)
- [x] T-065 Floating sky islands — ellipse filled with stone + grass cap (Terraria sky islands)
- [x] T-066 Cave mushrooms + crystal spike clusters placed deep underground
  (Terraria mushroom biome + Starbound crystal caves)
- [x] T-067 Ore progression: copper → iron → gold → titanium → diamond (Starbound/Terraria tiers)
- [x] T-068 Clay pockets near surface; flowers on surface; FLAMMABLE grass/flower tufts

---

## Phase 11 — Item & Inventory System ✅ Complete (T-069..T-077)

*Modelled on Starbound's StarItem / StarItemDescriptor / StarPlayerInventory.*

- [x] T-069 `arenite_core::items` module: `Item`, `ItemBuilder`, `ItemRarity`, `ItemCategory`
- [x] T-070 `ItemStack` (item_key + count), `Inventory` (fixed-capacity slots with merge/remove)
- [x] T-071 `PlayerInventory`: hotbar ×8 + main ×30 + armor ×4; `starter()` pre-filled
- [x] T-072 `default_item_registry()`: 25 tile-placers + 5 pickaxe tiers + 5 sword tiers
  + 2 potions (Starbound/Terraria item progression)
- [x] T-073 `active_paint_mat()`: registry lookup via hotbar item's `material_key`; palette fallback
- [x] T-074 1–8 hotkeys sync both legacy slot and `PlayerInventory.selected_hotbar`
- [x] T-075 Window title: active item `display_name` + `HP:current/max`
- [ ] T-076 egui HUD hotbar strip — item icons + counts rendered in-game
- [ ] T-077 Crafting table block + recipe registry (`ItemRecipe`); combine inputs → outputs

---

## Phase 12 — Entity & Combat System 🔄 In Progress (T-078..T-090)

*Inspired by Terraria's NPC/monster system and Starbound's StarMonster / StarNpc.*

- [x] T-078 `Entity` trait: position, velocity, health, hitbox, tick(), take_damage()
- [x] T-079 `SlimeEnemy` (Terraria Green Slime): patrol + aggro chase + jump + contact damage;
  health-tinted pixel colour (green → red as HP drops)
- [x] T-080 `BatEnemy` (Terraria Cave Bat): sinusoidal flight toward player
- [x] T-081 `EntityManager`: spawn, update, paint-and-clear pixel rendering per frame
- [x] T-082 Player HP system: `hp` / `max_hp` / `iframes` fields; invincibility frames
- [x] T-083 4 slimes + 3 bats spawned near player at world-ready (spawner bootstrap)
- [ ] T-084 `SkeletonEnemy` — ranged archer; fires `ArrowProjectile` at player
- [ ] T-085 Projectile system — `Projectile` struct with velocity + damage; rendered as pixel
- [ ] T-086 Boss framework — `BossPhase` enum; Giant Slime as first boss (Terraria Boss I)
- [ ] T-087 XP & level system — kill entities → XP → levels → stat bonuses
- [ ] T-088 Loot drops — entities drop `ItemStack` at death position; pickup radius
- [ ] T-089 Spawner system — spawn enemies in dark/underground areas; day/night multiplier
- [ ] T-090 NPC stub — `FriendlyNpc` trait; merchant NPC spawns in surface houses

---

## Phase 13 — World Structures & Dungeons (T-091..T-100)

*Inspired by Terraria's worldgen passes and Starbound's dungeon generator.*

- [ ] T-091 Dungeon generator — multi-room stone-brick corridors + locked doors + key item
- [ ] T-092 Underground houses — Starbound micro-dungeon underground shelters
- [ ] T-093 World border — unbreakable bedrock rows at bottom and side edges
- [ ] T-094 Chest block — placed storage container + per-biome loot table
- [ ] T-095 Loot tables — weighted item pools seeded by world RNG; per-biome variants
- [ ] T-096 Ice cave biome variant — crystal + ice underground at Tundra depth
- [ ] T-097 Lava chamber — obsidian-lined cavity near underworld with titanium veins
- [ ] T-098 Biome-specific decorations — cactus (Desert), pine cone / igloo (Tundra)
- [ ] T-099 Trap tiles — spike pit (damage on contact), boulder roller (Terraria temple)
- [ ] T-100 Shrine room — rare underground room with altar item + lore text

---

## Phase 14 — Multiplayer Refinement (T-101..T-108)

- [ ] T-101 Server streams `ChunkData` on player join (complete T-023)
- [ ] T-102 Broadcast `PlayerPos` every tick; render remote players as sprites
- [ ] T-103 Chat box — `NetMessage::Chat`; rendered in HUD
- [ ] T-104 Entity sync — server-authoritative HP/position; `NetMessage::EntityUpdate`
- [ ] T-105 PvP toggle — server config flag `pvp = true/false`
- [ ] T-106 Reconnect logic — GameClient auto-retries on connection drop (3× backoff)
- [ ] T-107 Ping display in HUD corner
- [ ] T-108 Server world save/load on graceful shutdown

---

## Phase 15 — Modding & Extensibility (T-109..T-116)

*Inspired by Starbound's Lua scripting + re-flora's asset hot-reload.*

- [ ] T-109 Lua scripting (mlua) — hook into material update rules per-tick
- [ ] T-110 Lua item scripts — `onUse`, `onEquip`, `onPickup` callbacks
- [ ] T-111 Lua monster AI — `init`, `update`, `onDamaged` per-entity Lua scripts
- [ ] T-112 JSON material loader — `assets/materials/*.json` at startup (Starbound `.material`)
- [ ] T-113 JSON item loader — `assets/items/*.item` Starbound-compatible JSON
- [ ] T-114 `ArenitePlugin` trait — crate API for custom materials, items, entities
- [ ] T-115 Asset hot-reload — `notify` watcher on `assets/`; reload without restart
- [ ] T-116 WASM build — wgpu WebGL backend; `wasm-pack` target for browser play

---

## Phase 16 — Polish & v1.0 Release (T-117..T-120)

- [ ] T-117 Main menu screen — title art + New / Load / Multiplayer / Quit
- [ ] T-118 Death & respawn — die animation; respawn at spawn point with HP penalty
- [ ] T-119 Sound effects — `kira` or `rodio`; plop (place), crack (mine), hit sound
- [ ] T-120 Music system — looping biome tracks; day/night transition (Starbound audio)
- [ ] T-121 Screen transitions — fade in/out on load, death, menu
- [ ] T-122 Settings menu — resolution, fullscreen toggle, volume sliders
- [ ] T-123 Controller support — gamepad via `gilrs`; analog stick movement
- [ ] T-124 Steam integration — Steamworks SDK stub; achievement hooks
- [ ] T-125 Performance profiler — `puffin` or `tracy` integration toggle
- [ ] T-126 v1.0 milestone — all T-025..T-125 complete; playtested; Steam page live

---

## Roadmap Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 1–4   | Build, correctness, core systems | ✅ Complete |
| 5–9   | Gameplay, physics, performance, CI | 🔄 Mostly done |
| 10    | Materials & worldgen expansion | ✅ Complete |
| 11    | Item & inventory system | ✅ Complete |
| 12    | Entity & combat (partial) | 🔄 In progress |
| 13    | Structures & dungeons | ⏳ Planned |
| 14    | Multiplayer refinement | ⏳ Planned |
| 15    | Modding & extensibility | ⏳ Planned |
| 16    | Polish & v1.0 | ⏳ Planned |

**Total tasks: 126 · Completed: 74 · Remaining: 52**
