use crate::chunk::ChunkData;
use crate::material::MaterialInstance;
use crate::particle::Particle;
use crate::simulator::{SimContext, Simulator};
use ahash::AHashMap;
use arenite_core::pos::{ChunkPos, TilePos, CHUNK_SIZE};
use std::cell::UnsafeCell;

/// Radius of chunks kept active around the player (fully simulated).
pub const ACTIVE_RADIUS: i32 = 4;
/// Radius of loaded-but-sleeping chunks (kept in memory, not simulated).
pub const LOAD_RADIUS: i32 = 8;
/// Chunks farther than this are evicted from memory entirely (T-047).
pub const UNLOAD_RADIUS: i32 = 12;

/// The simulation world: owns all chunks and drives tick-by-tick updates.
///
/// Design adapted from FallingSandEngine's `ChunkHandler` + `World`:
/// - Infinite world via `AHashMap<ChunkPos, Chunk>`
/// - Active chunks (near player) are ticked every frame
/// - Dirty rects track which regions need GPU re-upload
pub struct SimWorld {
    pub chunks: AHashMap<ChunkPos, Box<UnsafeCell<ChunkData>>>,
    /// Metadata (loaded, active flags) stored separately so we can iterate
    /// cleanly without the UnsafeCell getting in the way.
    pub meta: AHashMap<ChunkPos, ChunkMeta>,
    pub particles: Vec<Particle>,
    /// Current tick counter.
    pub tick: u64,
    /// Seed used for world generation.
    pub seed: u64,
    /// Centre position for chunk loading decisions.
    pub player_chunk: ChunkPos,
}

#[derive(Clone, Debug)]
pub struct ChunkMeta {
    pub pos: ChunkPos,
    pub loaded: bool,
    pub active: bool,
    pub last_tick: u64,
}

impl SimWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            chunks: AHashMap::default(),
            meta: AHashMap::default(),
            particles: Vec::new(),
            tick: 0,
            seed,
            player_chunk: ChunkPos::new(0, 0),
        }
    }

    // ── Chunk management ──────────────────────────────────────────────────

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.meta.get(&pos).is_some_and(|m| m.loaded)
    }

    /// Insert a freshly-generated chunk into the world.
    pub fn insert_chunk(&mut self, pos: ChunkPos, data: ChunkData) {
        self.chunks.insert(pos, Box::new(UnsafeCell::new(data)));
        self.meta.insert(
            pos,
            ChunkMeta {
                pos,
                loaded: true,
                active: true,
                last_tick: self.tick,
            },
        );
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
        self.meta.remove(&pos);
    }

    /// Update chunk active/loaded flags and evict far-away chunks (T-047).
    pub fn update_load_state(&mut self, player_chunk: ChunkPos) {
        self.player_chunk = player_chunk;
        for meta in self.meta.values_mut() {
            let dist = meta.pos.manhattan_distance(player_chunk);
            meta.active = dist <= ACTIVE_RADIUS;
            meta.loaded = dist <= LOAD_RADIUS;
        }
        // T-047: Evict chunks beyond UNLOAD_RADIUS.
        let evict: Vec<ChunkPos> = self
            .meta
            .iter()
            .filter(|(_, m)| m.pos.manhattan_distance(player_chunk) > UNLOAD_RADIUS)
            .map(|(pos, _)| *pos)
            .collect();
        for pos in evict {
            self.chunks.remove(&pos);
            self.meta.remove(&pos);
        }
    }

    // ── Pixel access ──────────────────────────────────────────────────────

    pub fn get_pixel(&self, pos: TilePos) -> MaterialInstance {
        let cp = pos.to_chunk();
        match self.chunks.get(&cp) {
            Some(cell) => {
                let (lx, ly) = pos.local();
                unsafe { (*cell.get()).get(lx, ly) }
            }
            None => MaterialInstance::air(),
        }
    }

    pub fn set_pixel(&mut self, pos: TilePos, mat: MaterialInstance) {
        let cp = pos.to_chunk();
        if let Some(cell) = self.chunks.get(&cp) {
            let (lx, ly) = pos.local();
            unsafe { (*cell.get()).set(lx, ly, mat) };
        }
    }

    /// Set a filled circle of pixels.
    pub fn paint_circle(&mut self, centre: TilePos, radius: i32, mat: MaterialInstance) {
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= r2 {
                    self.set_pixel(TilePos::new(centre.x + dx, centre.y + dy), mat);
                }
            }
        }
    }

    // ── Simulation tick ───────────────────────────────────────────────────

    /// Advance the simulation by one tick.
    ///
    /// Chunks are divided into 4 colour-class phases by (cx%2, cy%2) so no two
    /// chunks in the same phase share a write boundary — enabling safe parallel
    /// execution within each phase (T-021 / T-049).
    pub fn tick_simulation(&mut self) {
        self.tick += 1;
        let t = self.tick;

        // Bucket active chunks into 4 independent phases.
        let mut phases: [Vec<ChunkPos>; 4] = Default::default();
        for m in self.meta.values() {
            if !m.active || !m.loaded {
                continue;
            }
            let phase = (m.pos.x.rem_euclid(2) + m.pos.y.rem_euclid(2) * 2) as usize;
            phases[phase].push(m.pos);
        }

        let mut new_particles: Vec<Particle> = Vec::with_capacity(64);
        for phase in &phases {
            for &cp in phase {
                // T-048: skip fully-static chunks (no dynamic pixels).
                if let Some(cell) = self.chunks.get(&cp) {
                    if unsafe { (*cell.get()).dynamic_count == 0 } {
                        continue;
                    }
                }
                let mut spawned = self.tick_one_chunk(cp, t);
                new_particles.append(&mut spawned);
            }
        }
        self.particles.append(&mut new_particles);

        // Tick free particles.
        let chunks = &self.chunks;
        self.particles.retain_mut(|p| {
            if !p.tick() {
                return false;
            }
            // Try to re-embed settled particles into the world.
            if fastrand::u8(..) < 10 {
                let tp = TilePos::new(p.tile_x(), p.tile_y());
                if let Some(cell) = chunks.get(&tp.to_chunk()) {
                    let (lx, ly) = tp.local();
                    let dest = unsafe { (*cell.get()).get(lx, ly) };
                    if dest.is_air() {
                        unsafe { (*cell.get()).set(lx, ly, p.material) };
                        return false;
                    }
                }
            }
            true
        });
    }

    fn tick_one_chunk(&self, cp: ChunkPos, tick: u64) -> Vec<Particle> {
        // Build the 3×3 neighbourhood.
        let neighbours: [Option<&UnsafeCell<ChunkData>>; 9] = {
            let offsets: [(i32, i32); 9] = [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (0, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ];
            let mut arr: [Option<&UnsafeCell<ChunkData>>; 9] = [None; 9];
            for (i, &(dx, dy)) in offsets.iter().enumerate() {
                let np = ChunkPos::new(cp.x + dx, cp.y + dy);
                arr[i] = self.chunks.get(&np).map(|b| b.as_ref());
            }
            arr
        };

        // SAFETY: Sequential execution — no aliasing between chunks in this impl.
        // The quad-parallel scheduler (T-015) adds the non-overlapping guarantee
        // for parallel execution.
        let mut particles_local: Vec<Particle> = Vec::new();
        let mut ctx = SimContext::new(neighbours, &mut particles_local);
        Simulator::tick_chunk(&mut ctx, tick);
        particles_local
    }

    // ── Statistics ────────────────────────────────────────────────────────

    pub fn loaded_chunk_count(&self) -> usize {
        self.meta.values().filter(|m| m.loaded).count()
    }

    pub fn active_chunk_count(&self) -> usize {
        self.meta.values().filter(|m| m.active).count()
    }

    pub fn total_pixel_count(&self) -> usize {
        self.chunks.len() * (CHUNK_SIZE * CHUNK_SIZE) as usize
    }
}

// SAFETY: We only access UnsafeCell<ChunkData> from the simulation thread
// (or via the quad scheduler that prevents aliasing).
unsafe impl Send for SimWorld {}
unsafe impl Sync for SimWorld {}
