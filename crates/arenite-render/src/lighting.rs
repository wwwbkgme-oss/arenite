use arenite_core::pos::{CHUNK_SIZE, CHUNK_AREA};
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
    pub sky_color:    [f32; 3],
    pub attenuation:  f32,   // factor per tile step (e.g. 0.85)
}

impl Default for LightPropagator {
    fn default() -> Self {
        Self { sky_color: [1.0, 0.98, 0.95], attenuation: 0.82 }
    }
}

impl LightPropagator {
    /// Propagate light within a single chunk.
    /// `above_light` is the light row entering from the chunk above.
    pub fn propagate_chunk(
        &self,
        data:        &mut ChunkData,
        surface_row: Option<i32>,   // chunk-local y row of the surface (sky entry point)
    ) {
        let size = CHUNK_SIZE as usize;

        // Reset light buffer.
        for l in data.light.iter_mut() {
            *l = [0.0; 3];
        }

        // ── Sky light: rain downward from the top if this chunk is in the sky ──
        if let Some(surface) = surface_row {
            for x in 0..CHUNK_SIZE {
                let start_y = 0_i32.max(surface);
                for y in start_y..CHUNK_SIZE {
                    let idx = ChunkData::idx(x, y);
                    let px  = data.pixels[idx];
                    if !px.is_air() { break; }
                    // Sky light diminishes with depth below surface.
                    let depth_fac = 1.0 - (y - start_y) as f32 * 0.03;
                    let fac = depth_fac.max(0.1);
                    for c in 0..3 {
                        data.light[idx][c] = data.light[idx][c].max(self.sky_color[c] * fac);
                    }
                }
            }
        }

        // ── Emissive seed pass ──
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = ChunkData::idx(x, y);
                let em  = data.pixels[idx].light;
                for c in 0..3 {
                    if em[c] > 0.0 {
                        data.light[idx][c] = data.light[idx][c].max(em[c]);
                    }
                }
            }
        }

        // ── Spread pass (4 directions, repeated for stability) ──
        for _ in 0..12 {
            // Forward pass (top-left → bottom-right).
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let idx = ChunkData::idx(x, y);
                    let cur = data.light[idx];

                    for (nx, ny) in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)] {
                        if nx < 0 || nx >= CHUNK_SIZE || ny < 0 || ny >= CHUNK_SIZE {
                            continue;
                        }
                        let nidx = ChunkData::idx(nx, ny);
                        let target = data.pixels[nidx];
                        // Light passes through air and liquid, blocked by dense solids.
                        let pass = match target.physics {
                            arenite_sim::PhysicsType::Solid  => 0.15 * self.attenuation,
                            arenite_sim::PhysicsType::Sand   => 0.05 * self.attenuation,
                            _                                => self.attenuation,
                        };
                        for c in 0..3 {
                            let spread = cur[c] * pass;
                            if spread > data.light[nidx][c] {
                                data.light[nidx][c] = spread;
                            }
                        }
                    }
                }
            }
        }
    }
}
