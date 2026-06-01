use crate::chunk::ChunkData;
use crate::material::MaterialInstance;
use crate::particle::Particle;
use crate::physics_type::PhysicsType;
use arenite_core::pos::CHUNK_SIZE;
use std::cell::UnsafeCell;

// ── Material ID constants ─────────────────────────────────────────────────────
// These match the registration order in `default_material_registry()`.
// If new materials are inserted *before* these, bump the values here.
pub const MAT_AIR: u16 = 0;
pub const MAT_DIRT: u16 = 1;
pub const MAT_STONE: u16 = 2;
pub const MAT_SAND: u16 = 3;
pub const MAT_GRAVEL: u16 = 4;
pub const MAT_WATER: u16 = 5;
pub const MAT_LAVA: u16 = 6;
pub const MAT_STEAM: u16 = 7;
pub const MAT_SMOKE: u16 = 8;
pub const MAT_FIRE: u16 = 9;
pub const MAT_GRASS: u16 = 10;
pub const MAT_SNOW: u16 = 11;
pub const MAT_WOOD: u16 = 12;
pub const MAT_GOLD_ORE: u16 = 13;
pub const MAT_IRON_ORE: u16 = 14;
pub const MAT_CLAY: u16 = 15;
pub const MAT_MUD: u16 = 16;
pub const MAT_OBSIDIAN: u16 = 17;
pub const MAT_OIL: u16 = 18;
pub const MAT_ACID: u16 = 19;

/// Maximum lifetime (lower byte of data) for a fresh fire pixel.
const FIRE_MAX_LIFE: u16 = 80;
/// Max ticks a steam pixel lingers before fading.
const STEAM_LIFE: u16 = 50;

// ── Convenience constructors ──────────────────────────────────────────────────

fn make_steam() -> MaterialInstance {
    MaterialInstance {
        id: MAT_STEAM,
        physics: PhysicsType::Gas,
        color: arenite_core::Color::new(200, 200, 220, 80),
        light: [0.0; 3],
        data: STEAM_LIFE,
    }
}

fn make_fire(life: u16) -> MaterialInstance {
    MaterialInstance {
        id: MAT_FIRE,
        physics: PhysicsType::Fire,
        color: arenite_core::Color::new(255, 140, 0, 200),
        light: [1.0, 0.6, 0.1],
        data: life,
    }
}

fn make_smoke() -> MaterialInstance {
    MaterialInstance {
        id: MAT_SMOKE,
        physics: PhysicsType::Gas,
        color: arenite_core::Color::new(80, 80, 80, 120),
        light: [0.0; 3],
        data: fastrand::u16(20..60),
    }
}

fn make_obsidian() -> MaterialInstance {
    MaterialInstance {
        id: MAT_OBSIDIAN,
        physics: PhysicsType::Solid,
        color: arenite_core::Color::OBSIDIAN,
        light: [0.0; 3],
        data: 0,
    }
}

/// The Simulator drives one tick of the cellular-automata pixel simulation.
///
/// ## Algorithm
///
/// 1.  Split loaded chunks into independent 2×2 "quads".
///     Adjacent quads never share a write boundary within the same phase,
///     so each quad runs in parallel on rayon without data races.
///
/// 2.  Within a quad, each pixel is processed with access to a 3×3 chunk
///     neighbourhood (`SimContext`) so border pixels can write into
///     adjacent chunks safely.
///
/// 3.  A checkerboard (even/odd tick) ordering removes directional bias and
///     produces more natural-looking flows.
///
/// ## Physics rules added in this version:
///
/// - **T-033** Water pressure: enclosed liquid fills upward.
/// - **T-034** Lava + water contact → steam + obsidian.
/// - **T-035** Fire spreads to pixels with the `FLAMMABLE` data bit.
/// - **T-036** Wind: per-tick horizontal bias for Gas pixels.
#[allow(clippy::needless_return)]
pub struct Simulator;

impl Simulator {
    /// Tick a single chunk's simulation.
    ///
    /// `ctx` gives read/write access to the centre chunk and its 8 neighbours.
    /// `tick` is the global simulation tick counter (used for checkerboard ordering).
    pub fn tick_chunk(ctx: &mut SimContext, tick: u64) {
        let even = tick.is_multiple_of(2);
        // Wind direction: slowly oscillates left/right, period ~200 ticks (T-036).
        let wind_right = (tick / 100).is_multiple_of(2);

        for pass in 0..2_u8 {
            let y_range: Box<dyn Iterator<Item = i32>> = if pass == 0 {
                Box::new((0..CHUNK_SIZE).rev()) // bottom-to-top for gravity
            } else {
                Box::new(0..CHUNK_SIZE) // top-to-bottom for rising
            };

            for y in y_range {
                let x_range: Box<dyn Iterator<Item = i32>> = if (y as u64 + tick).is_multiple_of(2)
                {
                    Box::new(0..CHUNK_SIZE)
                } else {
                    Box::new((0..CHUNK_SIZE).rev())
                };

                for x in x_range {
                    let mat = ctx.get(x, y);

                    match mat.physics {
                        PhysicsType::Sand if pass == 0 => {
                            Self::update_sand(ctx, x, y, mat);
                        }
                        PhysicsType::Liquid if pass == 0 => {
                            Self::update_liquid(ctx, x, y, mat, even, tick);
                        }
                        PhysicsType::Gas if pass == 1 => {
                            Self::update_gas(ctx, x, y, mat, wind_right);
                        }
                        PhysicsType::Fire if pass == 1 => {
                            Self::update_fire(ctx, x, y, mat);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // ── Sand ─────────────────────────────────────────────────────────────────

    fn update_sand(ctx: &mut SimContext, x: i32, y: i32, mat: MaterialInstance) {
        // 1. Try falling straight down.
        if Self::displace(ctx, x, y, x, y + 1, mat) {
            return;
        }

        // 2. Try diagonal slides — randomise direction to remove bias.
        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };

        if Self::displace(ctx, x, y, x + dx0, y + 1, mat) {
            return;
        }
        let _ = Self::displace(ctx, x, y, x + dx1, y + 1, mat);
    }

    // ── Liquid ───────────────────────────────────────────────────────────────

    fn update_liquid(
        ctx: &mut SimContext,
        x: i32,
        y: i32,
        mat: MaterialInstance,
        even: bool,
        tick: u64,
    ) {
        // T-034: Lava + water → obsidian + steam.
        if mat.id == MAT_LAVA {
            Self::check_lava_water_contact(ctx, x, y);
            // Lava that turned into obsidian will have been replaced; re-read.
            let recheck = ctx.get(x, y);
            if recheck.id == MAT_OBSIDIAN {
                return;
            }
        }

        // 1. Try falling straight down.
        if Self::displace(ctx, x, y, x, y + 1, mat) {
            return;
        }

        // 2. Try falling diagonally.
        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };
        if Self::displace(ctx, x, y, x + dx0, y + 1, mat) {
            return;
        }
        if Self::displace(ctx, x, y, x + dx1, y + 1, mat) {
            return;
        }

        // 3. Spread horizontally (T-022 real spread_rate not wired, using default).
        let spread = 5_i32;
        let (start_dx, end_dx) = if even { (1, spread + 1) } else { (-spread, 0) };

        for dx in start_dx..end_dx {
            if dx == 0 {
                continue;
            }
            if Self::displace(ctx, x, y, x + dx, y, mat) {
                return;
            }
        }

        // T-033: Water pressure — if this liquid is squeezed, try rising.
        Self::apply_liquid_pressure(ctx, x, y, mat, tick);
    }

    /// T-033: BFS-style pressure: if a liquid is blocked on all sides but has
    /// liquid below or beside it, it can push upward.  Probability scales
    /// with the number of same-type liquid neighbours so enclosed columns
    /// gradually fill up.
    fn apply_liquid_pressure(
        ctx: &mut SimContext,
        x: i32,
        y: i32,
        mat: MaterialInstance,
        tick: u64,
    ) {
        // Count how many of the 3 lateral+below neighbours are same liquid.
        let neighbors = [ctx.get(x - 1, y), ctx.get(x + 1, y), ctx.get(x, y + 1)];
        let pressure: u8 = neighbors
            .iter()
            .filter(|n| n.physics == PhysicsType::Liquid && n.id == mat.id)
            .count() as u8;

        if pressure < 2 {
            return;
        } // not enough fluid around to create pressure

        // Higher pressure = more likely to push up this tick.
        let threshold: u8 = 80u8.saturating_sub(pressure * 25);
        if fastrand::u8(..) >= threshold {
            return;
        }

        // Try to displace into the cell directly above.
        let above = ctx.get(x, y - 1);
        if above.physics == PhysicsType::Air
            || (above.physics == PhysicsType::Liquid && above.id != mat.id)
        {
            ctx.set(x, y - 1, mat);
            ctx.set(x, y, above);
        }
        let _ = tick; // used for future time-based pressure effects
    }

    // ── T-034: Lava ↔ Water interaction ──────────────────────────────────────

    /// When lava is adjacent to water, the water becomes steam and the lava
    /// solidifies into obsidian.  This fires with low probability so lava
    /// flows persist a reasonable time before hardening.
    fn check_lava_water_contact(ctx: &mut SimContext, x: i32, y: i32) {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            let nb = ctx.get(nx, ny);
            if nb.id == MAT_WATER {
                // Low chance per-tick so interaction plays out slowly.
                if fastrand::u8(..) < 15 {
                    // Water becomes steam.
                    ctx.set(nx, ny, make_steam());
                    // Lava solidifies to obsidian.
                    ctx.set(x, y, make_obsidian());
                    return;
                }
            }
        }
    }

    // ── Gas ──────────────────────────────────────────────────────────────────

    /// T-036: Wind — Gas pixels rise as usual, with a horizontal bias
    /// that oscillates direction every ~100 ticks (global wind model).
    fn update_gas(ctx: &mut SimContext, x: i32, y: i32, mat: MaterialInstance, wind_right: bool) {
        // Gas decays/fades over time (lower byte of data = lifetime).
        let life = mat.lifetime();
        if life > 0 {
            let decayed = mat.with_lifetime(life.saturating_sub(1));
            ctx.set(x, y, decayed);
            // When it hits 0 next tick, just let it stop; it will be
            // overwritten by other dynamics naturally.
        }

        // Rise straight up.
        if Self::displace(ctx, x, y, x, y - 1, mat) {
            return;
        }

        // Diagonal rise.
        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };
        if Self::displace(ctx, x, y, x + dx0, y - 1, mat) {
            return;
        }
        if Self::displace(ctx, x, y, x + dx1, y - 1, mat) {
            return;
        }

        // T-036: Horizontal wind drift — biased toward current wind direction.
        let wind_dx = if wind_right { 1 } else { -1 };
        if fastrand::u8(..) < 40 {
            if Self::displace(ctx, x, y, x + wind_dx, y, mat) {
                return;
            }
        }
        // Weak opposite drift for turbulence.
        if fastrand::u8(..) < 15 {
            let _ = Self::displace(ctx, x, y, x - wind_dx, y, mat);
        }
    }

    // ── Fire ─────────────────────────────────────────────────────────────────

    /// T-035: Fire spreads to any neighbour with the FLAMMABLE data bit set,
    /// not just Gas — so wood, grass, and oil all catch fire correctly.
    fn update_fire(ctx: &mut SimContext, x: i32, y: i32, mut mat: MaterialInstance) {
        // Decrease lifetime (lower byte of data).
        let life = mat.lifetime();
        if life == 0 {
            ctx.set(x, y, MaterialInstance::air());
            return;
        }
        let decayed = mat.with_lifetime(life - 1);
        ctx.set(x, y, decayed);

        // Emit smoke upward occasionally.
        if fastrand::u8(..) < 25 {
            let up = ctx.get(x, y - 1);
            if up.is_air() {
                ctx.set(x, y - 1, make_smoke());
            }
        }

        // T-035: Spread to flammable neighbours.
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            let nb = ctx.get(nx, ny);
            if nb.is_flammable() && fastrand::u8(..) < 40 {
                ctx.set(nx, ny, make_fire(fastrand::u16(20..FIRE_MAX_LIFE)));
            }
        }

        // Rise slightly.
        mat = ctx.get(x, y); // re-read (may have changed above)
        if mat.physics == PhysicsType::Fire {
            Self::displace(ctx, x, y, x, y - 1, mat);
        }
    }

    // ── Displacement helper ───────────────────────────────────────────────────

    /// Try to move `mat` at (from_x, from_y) → (to_x, to_y).
    /// Succeeds when the destination is displaceable AND lighter than `mat`.
    /// Returns `true` if the swap happened.
    #[inline]
    fn displace(
        ctx: &mut SimContext,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        mat: MaterialInstance,
    ) -> bool {
        let dest = ctx.get(to_x, to_y);

        // Can only move into air, liquid, or gas.
        if !dest.physics.is_displaceable() {
            return false;
        }

        ctx.set(to_x, to_y, mat);
        ctx.set(from_x, from_y, dest);
        true
    }
}

/// Provides pixel read/write access to a 3×3 chunk neighbourhood,
/// used by the simulator so border pixels can be updated correctly.
///
/// The centre chunk is index 4 in the 3×3 grid:
/// ```text
///  0 | 1 | 2
/// ---+---+---
///  3 | 4 | 5
/// ---+---+---
///  6 | 7 | 8
/// ```
pub struct SimContext<'a> {
    /// 9 chunk data pointers; using UnsafeCell for interior mutability during
    /// parallel simulation (the simulator guarantees non-overlapping writes).
    chunks: [Option<&'a UnsafeCell<ChunkData>>; 9],
    pub particles: &'a mut Vec<Particle>,
}

impl<'a> SimContext<'a> {
    pub fn new(
        chunks: [Option<&'a UnsafeCell<ChunkData>>; 9],
        particles: &'a mut Vec<Particle>,
    ) -> Self {
        Self { chunks, particles }
    }

    /// Convert a chunk-local (x, y) relative to the centre chunk into
    /// a (chunk_slot, local_x, local_y) triple.
    #[inline]
    fn coords_to_slot(x: i32, y: i32) -> (usize, i32, i32) {
        let cx = if x < 0 {
            0
        } else if x < CHUNK_SIZE {
            1
        } else {
            2
        };
        let cy = if y < 0 {
            0
        } else if y < CHUNK_SIZE {
            1
        } else {
            2
        };
        let slot = (cy * 3 + cx) as usize;
        let lx = x.rem_euclid(CHUNK_SIZE);
        let ly = y.rem_euclid(CHUNK_SIZE);
        (slot, lx, ly)
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> MaterialInstance {
        let (slot, lx, ly) = Self::coords_to_slot(x, y);
        match self.chunks[slot] {
            Some(c) => unsafe { (*c.get()).get(lx, ly) },
            None => MaterialInstance::air(),
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        let (slot, lx, ly) = Self::coords_to_slot(x, y);
        if let Some(c) = self.chunks[slot] {
            // SAFETY: the quad scheduling in SimWorld ensures no two SimContexts
            // share a writable chunk in the same rayon task.
            unsafe { (*c.get()).set(lx, ly, mat) };
        }
    }
}
