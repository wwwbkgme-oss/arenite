use crate::noise_field::NoiseField;
use arenite_core::pos::TilePos;
use arenite_sim::material::MaterialInstance;
use arenite_sim::SimWorld;

/// Cave carver using domain-warped FBM noise.
///
/// Technique from FallingSandEngine's cave populator:
/// two overlapping noise layers, both squared, giving
/// sharp-walled tunnels with natural variation.
pub struct CaveCarver {
    pub threshold_underground: f64,
    pub threshold_cavern: f64,
    pub threshold_underworld: f64,
}

impl Default for CaveCarver {
    fn default() -> Self {
        Self {
            threshold_underground: 0.28,
            threshold_cavern: 0.22,
            threshold_underworld: 0.35,
        }
    }
}

/// Context passed to cave-carve operations (reduces argument count).
pub struct CaveContext<'a> {
    pub world_width: i32,
    pub world_height: i32,
    pub noise: &'a NoiseField,
    pub underground_y: i32,
    pub cavern_y: i32,
    pub cave_factor: f64,
}

impl CaveCarver {
    /// Return true if the pixel at (x, y) should be carved to air.
    pub fn should_carve(&self, x: i32, y: i32, surface_y: i32, ctx: &CaveContext<'_>) -> bool {
        if y <= surface_y {
            return false;
        }

        let nx = x as f64 / ctx.world_width as f64 * 6.0;
        let ny = y as f64 / ctx.world_height as f64 * 6.0;
        let cave_value = ctx.noise.cave(nx, ny);

        let depth_factor = if y > ctx.cavern_y {
            2.0
        } else if y > ctx.underground_y {
            1.4
        } else {
            1.0
        };

        let threshold = if y > ctx.cavern_y {
            self.threshold_cavern
        } else if y > ctx.underground_y {
            self.threshold_underground
        } else {
            self.threshold_underworld
        };

        cave_value < threshold * depth_factor * ctx.cave_factor
    }

    /// Carve all caves in a column range into the SimWorld.
    pub fn carve_region(
        &self,
        world: &mut SimWorld,
        x0: i32,
        x1: i32,
        surface: &[i32],
        ctx: &CaveContext<'_>,
    ) {
        for x in x0..x1 {
            let sy = surface
                .get(x as usize)
                .copied()
                .unwrap_or(ctx.world_height / 3);
            for y in sy..ctx.world_height {
                if self.should_carve(x, y, sy, ctx) {
                    world.set_pixel(TilePos::new(x, y), MaterialInstance::air());
                }
            }
        }
    }
}
