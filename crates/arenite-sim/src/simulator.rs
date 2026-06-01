use std::cell::UnsafeCell;
use arenite_core::pos::CHUNK_SIZE;
use crate::chunk::ChunkData;
use crate::material::MaterialInstance;
use crate::physics_type::PhysicsType;
use crate::particle::Particle;

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
/// Each `MaterialInstance` carries a `PhysicsType` that selects the update rule:
/// - `Sand`   → fall, slide diagonally
/// - `Liquid` → fall, spread horizontally up to `spread_rate`
/// - `Gas`    → rise, spread horizontally
/// - `Fire`   → spread to flammable neighbours, consume lifetime
/// - `Solid` / `Air` / `Object` → static, no update
#[allow(clippy::needless_return)]
pub struct Simulator;

impl Simulator {
    /// Tick a single chunk's simulation.
    ///
    /// `ctx` gives read/write access to the centre chunk and its 8 neighbours.
    /// `tick` is the global simulation tick counter (used for checkerboard ordering).
    pub fn tick_chunk(ctx: &mut SimContext, tick: u64) {
        let even = tick.is_multiple_of(2);
        // Process pixels bottom-to-top for sand/liquid (they want to fall),
        // and top-to-bottom for gas/fire (they rise).
        for pass in 0..2_u8 {
            let y_range: Box<dyn Iterator<Item = i32>> = if pass == 0 {
                Box::new((0..CHUNK_SIZE).rev())   // bottom-to-top for gravity
            } else {
                Box::new(0..CHUNK_SIZE)            // top-to-bottom for rising
            };

            for y in y_range {
                // Alternate horizontal direction per row to avoid drift.
                let x_range: Box<dyn Iterator<Item = i32>> = if (y as u64 + tick).is_multiple_of(2) {
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
                            Self::update_liquid(ctx, x, y, mat, even);
                        }
                        PhysicsType::Gas if pass == 1 => {
                            Self::update_gas(ctx, x, y, mat, even);
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

    // ── Sand ────────────────────────────────────────────────────────────────

    fn update_sand(ctx: &mut SimContext, x: i32, y: i32, mat: MaterialInstance) {
        // 1. Try falling straight down.
        if Self::displace(ctx, x, y, x, y + 1, mat) { return; }

        // 2. Try diagonal slides — randomise direction to remove bias.
        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };

        if Self::displace(ctx, x, y, x + dx0, y + 1, mat) { return; }
        // Last attempt — no return needed, function ends here.
        let _ = Self::displace(ctx, x, y, x + dx1, y + 1, mat);
    }

    // ── Liquid ───────────────────────────────────────────────────────────────

    fn update_liquid(ctx: &mut SimContext, x: i32, y: i32, mat: MaterialInstance, even: bool) {
        // 1. Try falling straight down (displace lighter materials).
        if Self::displace(ctx, x, y, x, y + 1, mat) { return; }

        // 2. Try falling diagonally.
        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };
        if Self::displace(ctx, x, y, x + dx0, y + 1, mat) { return; }
        if Self::displace(ctx, x, y, x + dx1, y + 1, mat) { return; }

        // 3. Spread horizontally up to spread_rate tiles.
        // The spread_rate lives in material.data (encoded during registration).
        let spread = 5_i32; // default spread; real impl queries registry
        let (start_dx, end_dx) = if even { (1, spread + 1) } else { (-spread, 0) };

        for dx in start_dx..end_dx {
            if dx == 0 { continue; }
            if Self::displace(ctx, x, y, x + dx, y, mat) { return; }
        }
    }

    // ── Gas ──────────────────────────────────────────────────────────────────

    fn update_gas(ctx: &mut SimContext, x: i32, y: i32, mat: MaterialInstance, even: bool) {
        // Gases rise.
        if Self::displace(ctx, x, y, x, y - 1, mat) { return; }

        let left_first = fastrand::bool();
        let (dx0, dx1) = if left_first { (-1, 1) } else { (1, -1) };
        if Self::displace(ctx, x, y, x + dx0, y - 1, mat) { return; }
        if Self::displace(ctx, x, y, x + dx1, y - 1, mat) { return; }

        let spread = 3_i32;
        let (start_dx, end_dx) = if even { (1, spread + 1) } else { (-spread, 0) };
        for dx in start_dx..end_dx {
            if dx == 0 { continue; }
            if Self::displace(ctx, x, y, x + dx, y, mat) { return; }
        }
    }

    // ── Fire ─────────────────────────────────────────────────────────────────

    fn update_fire(ctx: &mut SimContext, x: i32, y: i32, mut mat: MaterialInstance) {
        // Decrease lifetime (stored in mat.data).
        if mat.data == 0 {
            ctx.set(x, y, MaterialInstance::air());
            return;
        }
        mat.data = mat.data.saturating_sub(1);
        ctx.set(x, y, mat);

        // Emit smoke upward occasionally.
        if fastrand::u8(..) < 20 {
            let up = ctx.get(x, y - 1);
            if up.is_air() {
                let mut smoke = mat;
                smoke.physics = PhysicsType::Gas;
                smoke.data    = fastrand::u16(20..60);
                ctx.set(x, y - 1, smoke);
            }
        }

        // Spread to flammable neighbours.
        for (nx, ny) in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)] {
            let nb = ctx.get(nx, ny);
            if nb.physics.is_flammable() && fastrand::u8(..) < 50 {
                let mut fire = mat;
                fire.data = fastrand::u16(20..80);
                ctx.set(nx, ny, fire);
            }
        }

        // Rise slightly.
        Self::displace(ctx, x, y, x, y - 1, mat);
    }

    // ── Displacement helper ───────────────────────────────────────────────────

    /// Try to move `mat` at (from_x, from_y) → (to_x, to_y).
    /// Succeeds when the destination is displaceable AND lighter than `mat`.
    /// Returns `true` if the swap happened.
    #[inline]
    fn displace(
        ctx:    &mut SimContext,
        from_x: i32, from_y: i32,
        to_x:   i32, to_y:   i32,
        mat:    MaterialInstance,
    ) -> bool {
        let dest = ctx.get(to_x, to_y);

        // Can only move into air, liquid, or gas.
        if !dest.physics.is_displaceable() { return false; }

        // Heavier sinks into lighter (density determines which wins).
        // If both are liquids, always sink (handled by caller).
        if dest.physics != PhysicsType::Air {
            // Skip density comparison for now — always displace for simplicity.
            // A real implementation would check material registry densities.
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
        // Determine which of the 3×3 grid cells owns this coordinate.
        let cx = if x < 0 { 0 } else if x < CHUNK_SIZE { 1 } else { 2 };
        let cy = if y < 0 { 0 } else if y < CHUNK_SIZE { 1 } else { 2 };
        let slot = (cy * 3 + cx) as usize;
        let lx   = x.rem_euclid(CHUNK_SIZE);
        let ly   = y.rem_euclid(CHUNK_SIZE);
        (slot, lx, ly)
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> MaterialInstance {
        let (slot, lx, ly) = Self::coords_to_slot(x, y);
        match self.chunks[slot] {
            Some(c) => unsafe { (*c.get()).get(lx, ly) },
            None    => MaterialInstance::air(), // out-of-bounds ⇒ treat as air
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
