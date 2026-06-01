use arenite_core::pos::{ChunkPos, TilePos, CHUNK_SIZE};
use arenite_sim::SimWorld;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;
use crate::noise_field::NoiseField;

/// Cave carver using domain-warped FBM noise.
///
/// Technique from FallingSandEngine's cave populator:
/// two overlapping noise layers, both squared, giving
/// sharp-walled tunnels with natural variation.
pub struct CaveCarver {
    pub threshold_underground: f64,
    pub threshold_cavern:      f64,
    pub threshold_underworld:  f64,
}

impl Default for CaveCarver {
    fn default() -> Self {
        Self {
            threshold_underground: 0.28,
            threshold_cavern:      0.22,
            threshold_underworld:  0.35,
        }
    }
}

impl CaveCarver {
    /// Return true if the pixel at (x, y) should be carved to air.
    pub fn should_carve(
        &self,
        x: i32,
        y: i32,
        world_width:  i32,
        world_height: i32,
        noise:        &NoiseField,
        surface_y:    i32,
        underground_y:i32,
        cavern_y:     i32,
        cave_factor:  f64,
    ) -> bool {
        // No caves above the surface.
        if y <= surface_y { return false; }

        let nx = x as f64 / world_width  as f64 * 6.0;
        let ny = y as f64 / world_height as f64 * 6.0;

        let cave_value = noise.cave(nx, ny);

        // Cave density increases with depth.
        let depth_factor = if y > cavern_y {
            2.0   // deep caverns are larger
        } else if y > underground_y {
            1.4   // underground has medium caves
        } else {
            1.0   // shallow underground
        };

        let threshold = if y > cavern_y {
            self.threshold_cavern
        } else if y > underground_y {
            self.threshold_underground
        } else {
            self.threshold_underworld
        };

        cave_value < threshold * depth_factor * cave_factor
    }

    /// Carve all caves in a column range into the SimWorld.
    pub fn carve_region(
        &self,
        world:        &mut SimWorld,
        x0: i32, x1: i32,
        world_width:  i32,
        world_height: i32,
        noise:        &NoiseField,
        surface:      &[i32],
        underground_y:i32,
        cavern_y:     i32,
        cave_factor:  f64,
    ) {
        for x in x0..x1 {
            let sy = surface.get(x as usize).copied().unwrap_or(world_height / 3);
            for y in sy..world_height {
                if self.should_carve(
                    x, y,
                    world_width, world_height,
                    noise,
                    sy, underground_y, cavern_y,
                    cave_factor,
                ) {
                    world.set_pixel(TilePos::new(x, y), MaterialInstance::air());
                }
            }
        }
    }
}
