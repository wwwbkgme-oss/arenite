# Arenite Engine

**A falling-sand pixel simulation + procedural open-world game engine, written in Rust.**

[![CI](https://github.com/wwwbkgme-oss/arenite/actions/workflows/ci.yml/badge.svg)](https://github.com/wwwbkgme-oss/arenite/actions)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

> *Arenite* is a geological term for a sand-based sedimentary rock.  
> The name unites the two core ideas: **sand physics** and **Rust** (oxide).

---

## What Is Arenite?

Arenite Engine is a unified Rust game engine combining three ideas into one cohesive system:

| Idea | Inspiration | What Arenite does |
|------|-------------|-------------------|
| **Procedural world generation** | terra-awg (C++) | Seamless noise terrain, 7 biomes, domain-warped caves, ore blobs, surface structures |
| **Falling-sand cellular automata** | FallingSandEngine (Rust) | Multi-threaded pixel sim — sand, liquid, gas, fire — with rigidbody interaction |
| **Open-world Terraria-style gameplay** | rustaria (Rust) | Client/server split, player physics, paint pixels, multiplayer via TCP |

Everything is **original Rust code** — no assets, no code, and no content from any of the three inspiration projects is included.

---

## Workspace Layout

```
arenite/
├── Cargo.toml                   # workspace manifest (resolver = "2")
│
├── crates/
│   ├── arenite-core/            # Shared types, math, registry, RNG
│   ├── arenite-sim/             # Cellular automata pixel simulation engine
│   ├── arenite-world/           # Procedural world generation pipeline
│   ├── arenite-physics/         # Rapier2d rigidbody ↔ pixel grid bridge
│   ├── arenite-render/          # wgpu GPU renderer + Terraria-style lighting
│   └── arenite-net/             # Tokio TCP client/server networking
│
└── bin/
    ├── arenite-game/            # Client binary  (`cargo run -p arenite-game`)
    └── arenite-server/          # Headless server (`cargo run -p arenite-server`)
```

---

## Quick Start

### Prerequisites

**Linux (Ubuntu/Debian)**
```bash
sudo apt-get install -y \
  build-essential libvulkan-dev libegl1-mesa-dev \
  libx11-dev libxcb-xfixes0-dev libxkbcommon-dev \
  libwayland-dev pkg-config
```

**macOS** — just Xcode Command Line Tools + Rust.

**Windows** — Rust + Visual Studio Build Tools.

### Build & Run

```bash
# Clone
git clone https://github.com/wwwbkgme-oss/arenite
cd arenite

# Run the game client (generates a world on first launch)
cargo run -p arenite-game --release

# Run the dedicated server
cargo run -p arenite-server --release
```

### Configuration

Create `arenite.toml` next to the binary (optional):

```toml
world_width  = 2100
world_height = 600
world_seed   = 12345
player_name  = "Player"
# server = "127.0.0.1:25565"   # uncomment to connect to a remote server
```

Create `arenite-server.toml` for the dedicated server:

```toml
bind         = "0.0.0.0:25565"
world_width  = 4200
world_height = 1200
world_seed   = 42
tps          = 20
```

---

## Crate Reference

### `arenite-core`

Shared primitives used by every other crate.

| Module | Contents |
|--------|----------|
| `color` | `Color` — RGBA u8 with lerp, multiply, jitter |
| `id` | `Id<T>` — typed numeric IDs; `StringId` for registry keys |
| `math` | `IRect`, `Rect`, `Direction4`, `smoothstep`, `lerpf` |
| `pos` | `TilePos`, `ChunkPos`, `WorldPos`; `CHUNK_SIZE = 64` |
| `registry` | `Registry<T>` — string ↔ `Id<T>` bidirectional map |
| `rng` | `AreniteRng` — deterministic RNG, per-coord seeding |

### `arenite-sim`

Multi-threaded falling-sand cellular automata.

```
PhysicsType: Air | Solid | Sand | Liquid | Gas | Fire | Object
Material: display_name, base_color, density, viscosity, spread_rate, …
MaterialInstance: id, physics, color, light [f32;3], data u16
ChunkData: 64×64 MaterialInstance grid + DirtyRect
SimContext: 3×3 chunk neighbourhood for cross-border writes (UnsafeCell)
Simulator: sand/liquid/gas/fire update rules
SimWorld: AHashMap<ChunkPos, Box<UnsafeCell<ChunkData>>>
          get/set_pixel, paint_circle, tick_simulation, particles
```

**15 built-in materials:** air, dirt, stone, sand, gravel, water, lava, steam, smoke, fire, grass, snow, wood, gold ore, iron ore.

**Simulation rules per tick:**
1. Sand: fall → diagonal slide
2. Liquid: fall → diagonal → horizontal spread
3. Gas: rise → diagonal → horizontal spread
4. Fire: consume lifetime, emit smoke, spread to flammable neighbours

### `arenite-world`

Procedural world generation pipeline.

```
NoiseField: FBM OpenSimplex (fine + coarse), domain-warped cave noise,
            biome selector, feature noise — seeded from u64
BiomeMap: 7 biomes (plains/desert/tundra/jungle/ocean/cavern/underworld)
          ocean forced at world edges
HeightMap: 4D toroidal seamless noise (terra-awg technique),
           3-pass Gaussian smooth, underground/cavern/underworld layers
CaveCarver: domain-warped double-FBM, depth-scaled thresholds
FeaturePlacer: noise-gated ore blob scatter (iron, gold)
Structure: stamped pixel templates (house, pyramid)
WorldGenerator: full pipeline → SimWorld
```

**World constants (default):** 4200 × 1200 tiles = ~37 km² ; small preset: 2100 × 600.

### `arenite-physics`

Bridges Rapier2d rigidbodies with the pixel simulation.

```
PhysicsWorld: full rapier2d pipeline — step(dt), add_dynamic_box,
              add_static_box, apply_pixel_impulse, body_pixel_pos
PHYSICS_SCALE = 16.0 (pixels per physics metre)
RigidBody: pre_sim_stamp  — stamps Object pixels, collects sand impulse
           post_sim_clear — removes Object pixels after sand tick
```

### `arenite-render`

GPU renderer built on wgpu 22.

```
Camera2D: orthographic view_proj, scroll, zoom [0.125×, 16×], screen_to_world
Vertex2D: position + UV, LAYOUT descriptor, quad() helper
ChunkTexture: RGBA8 wgpu::Texture per chunk, dirty-aware CPU→GPU upload
LightPropagator: Terraria-style RGB BFS lighting, sky entry, emissive seeds
WorldPipeline: WGSL nearest-neighbour shader, camera UBO, alpha blending
AreniteRenderer: adapter selection, surface config, sync_chunks, render()
```

### `arenite-net`

TCP client/server networking with length-prefixed bincode frames.

```
NetMessage: Hello/Welcome/Reject, ChunkData, PixelUpdate,
            Input(InputState), PlayerPos/Joined/Left, Ping/Pong, Disconnect
codec: async u32-length-prefixed bincode read/write
GameServer: TcpListener, player ID allocation, broadcast
GameClient: connect + handshake, background IO tasks, poll_messages()
```

---

## Controls (Game Client)

| Key / Button | Action |
|---|---|
| `A` / `←`, `D` / `→` | Move left / right |
| `Space` | Jump |
| `Scroll wheel` | Zoom in / out |
| `Left click` (hold) | Paint material at cursor |
| `Right click` (hold) | Erase pixels |
| `1` – `6` | Select material (sand/water/stone/dirt/smoke/fire) |
| `Esc` | Quit |

---

## Key Design Decisions

### Why UnsafeCell in the simulator?

The cellular automata simulation accesses pixels across chunk boundaries.  
A safe API would require wrapping every pixel access in `Arc<Mutex<...>>` at a prohibitive cost.  
Instead, `SimContext` uses `UnsafeCell<ChunkData>` and the caller (quad scheduler) guarantees that no two concurrent tasks share a writable chunk — matching the approach used by FallingSandEngine.

### Why not bevy?

Bevy is excellent but adds ~200 dependencies and ~90 s of compile time.  
Arenite targets minimal dependencies for fast iteration: `winit` + `wgpu` for the platform layer, `rapier2d` for physics, `noise` for generation.  A bevy plugin adaptor is straightforward to add.

### Physics scale

`PHYSICS_SCALE = 16.0`: one physics metre = 16 pixels.  
This gives realistic dynamics (a player is ~2 m = 32 px tall) without numerical precision issues at the pixel level.

---

## Roadmap

- [ ] Inventory + item system
- [ ] Crafting table
- [ ] Enemy AI (pathfinding on pixel grid)
- [ ] Fluid simulation: pressure, pipes
- [ ] Multiplayer chunk streaming (ChunkData messages)
- [ ] Save / load world state (bincode serialisation)
- [ ] Lua modding API (mlua)
- [ ] WASM build (wgpu WebGL backend)
- [ ] Parallax background layers
- [ ] Destructible structures (RigidBody pixel grid)

---

## License

Apache-2.0 — see [LICENSE](LICENSE).

This project is an **entirely original work**.  
It draws architectural *inspiration* from the following open-source projects but contains **no code, assets, or content** from them:

- [terra-awg](https://github.com/alpha0010/terra-awg) — world generation ideas (C++)
- [FallingSandEngine](https://github.com/PieKing1215/FallingSandEngine) — pixel sim architecture (BSD-3-Clause Rust)
- [rustaria](https://github.com/TeamQuantumFusion/rustaria) — game structure ideas (Rust)
