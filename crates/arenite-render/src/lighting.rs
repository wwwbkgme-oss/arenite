use arenite_core::pos::CHUNK_SIZE;
use arenite_sim::chunk::ChunkData;

/// Terraria-style coloured light propagation.
///
/// Light spreads through transparent pixels (air, liquid, glass), attenuating
/// per-step.  Emissive pixels (lava, fire, glowing ores) seed the propagation.
/// Three separate channels (R, G, B) allow coloured light sources.
///
/// Algorithm (matches FallingSandEngine's lighting_propagate.comp logic,
/// implemented here on the CPU for portability):
///  1. Seed all emissive pixels with their emission value.
///  2. Seed surface pixels with sky light (full white from above).
///  3. BFS outward, multiplying by per-pixel attenuation.
///  4. Write results into ChunkData::light.
pub struct LightPropagator {
    pub sky_color:   [f32; 3],
    pub attenuation: f32,
}

impl Default for LightPropagator {
    fn default() -> Self {
        Self { sky_color: [1.0, 0.98, 0.95], attenuation: 0.82 }
    }
}

impl LightPropagator {
    /// Propagate light within a single chunk.
    /// `surface_row` is the chunk-local y of the sky entry point (if any).
    pub fn propagate_chunk(
        &self,
        data:        &mut ChunkData,
        surface_row: Option<i32>,
    ) {
        // Reset light buffer.
        for l in &mut data.light {
            *l = [0.0; 3];
        }

        // ── Sky light ─────────────────────────────────────────────────────────
        if let Some(surface) = surface_row {
            for x in 0..CHUNK_SIZE {
                let start_y = 0_i32.max(surface);
                for y in start_y..CHUNK_SIZE {
                    let idx = ChunkData::idx(x, y);
                    if !data.pixels[idx].is_air() { break; }
                    let fac = (1.0 - (y - start_y) as f32 * 0.03).max(0.1);
                    let l   = &mut data.light[idx];
                    for (lv, &sky) in l.iter_mut().zip(self.sky_color.iter()) {
                        *lv = lv.max(sky * fac);
                    }
                }
            }
        }

        // ── Emissive seed ──────────────────────────────────────────────────────
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = ChunkData::idx(x, y);
                let em  = data.pixels[idx].light;
                let l   = &mut data.light[idx];
                for i in 0..3 {
                    if em[i] > 0.0 { l[i] = l[i].max(em[i]); }
                }
            }
        }

        // ── Spread passes ─────────────────────────────────────────────────────
        for _ in 0..12 {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let idx = ChunkData::idx(x, y);
                    let cur = data.light[idx];

                    for (nx, ny) in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)] {
                        if !(0..CHUNK_SIZE).contains(&nx) || !(0..CHUNK_SIZE).contains(&ny) {
                            continue;
                        }
                        let nidx   = ChunkData::idx(nx, ny);
                        let target = data.pixels[nidx];
                        let pass   = match target.physics {
                            arenite_sim::PhysicsType::Solid => 0.15 * self.attenuation,
                            arenite_sim::PhysicsType::Sand  => 0.05 * self.attenuation,
                            _                               => self.attenuation,
                        };
                        let nlight = &mut data.light[nidx];
                        for (nv, &cv) in nlight.iter_mut().zip(cur.iter()) {
                            let spread = cv * pass;
                            if spread > *nv { *nv = spread; }
                        }
                    }
                }
            }
        }
    }
}
