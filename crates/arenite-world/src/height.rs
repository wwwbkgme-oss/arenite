use crate::noise_field::NoiseField;

/// A 1D surface heightmap per tile column.
///
/// The approach is taken from terra-awg's `World.cpp`:
/// - Seamless noise wrapping via toroidal 4D sampling
/// - Separate coarse + fine layers
/// - Per-biome height modifier applied on top
pub struct HeightMap {
    /// Surface y-tile for each world column.
    pub surface: Vec<i32>,
    /// Underground layer start (below surface dirt/stone transition).
    pub underground: i32,
    /// Cavern layer start.
    pub cavern: i32,
    /// Underworld start.
    pub underworld: i32,
}

impl HeightMap {
    pub fn generate(
        world_width: i32,
        world_height: i32,
        noise: &NoiseField,
        biome_height_mods: &[f64],
    ) -> Self {
        let mid = world_height as f64 * 0.35; // nominal surface y
        let amplitude = world_height as f64 * 0.08;

        let mut surface = Vec::with_capacity(world_width as usize);
        for x in 0..world_width {
            // Normalise to [0, 1] for noise sampling.
            let nx = x as f64 / world_width as f64;
            // 4D torus trick for seamless wrapping (terra-awg pattern):
            // map x → (cos(2πx), sin(2πx)) as two noise inputs.
            let angle = std::f64::consts::TAU * nx;
            let nx4 = angle.cos();
            let ny4 = angle.sin();

            let base = noise.terrain_coarse(nx4 * 0.5, ny4 * 0.5);
            let detail = noise.terrain_fine(nx4, ny4) * 0.35;

            let biome_mod = biome_height_mods.get(x as usize).copied().unwrap_or(0.0);

            let y = mid + (base + detail + biome_mod) * amplitude;
            surface.push(y.round() as i32);
        }

        // Smooth the surface to avoid single-pixel spikes (3-pass average).
        for _ in 0..3 {
            let mut smoothed = surface.clone();
            for x in 1..world_width as usize - 1 {
                smoothed[x] = (surface[x - 1] + surface[x] + surface[x + 1]) / 3;
            }
            surface = smoothed;
        }

        let underground = (world_height as f64 * super::UNDERGROUND_DEPTH) as i32;
        let cavern = (world_height as f64 * super::CAVERN_DEPTH) as i32;
        let underworld = world_height - super::UNDERWORLD_START;

        Self {
            surface,
            underground,
            cavern,
            underworld,
        }
    }

    pub fn surface_at(&self, x: i32) -> i32 {
        let w = self.surface.len() as i32;
        self.surface[x.rem_euclid(w) as usize]
    }

    /// Minimum surface elevation across all columns.
    pub fn min_surface(&self) -> i32 {
        *self.surface.iter().min().unwrap_or(&0)
    }

    /// Maximum surface elevation.
    pub fn max_surface(&self) -> i32 {
        *self.surface.iter().max().unwrap_or(&0)
    }
}
