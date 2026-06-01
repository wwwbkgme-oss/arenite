use rapier2d::prelude::*;
use arenite_core::pos::TilePos;
use arenite_sim::SimWorld;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;
use crate::physics::{PhysicsWorld, PHYSICS_SCALE};

/// A game entity backed by a rapier2d rigidbody.
///
/// Integrates with the pixel sim using the FallingSandEngine pattern:
///  1. Before the sand tick: stamp `Object` pixels into the world at the
///     body's AABB footprint.  Any sand pixel already there gets displaced
///     into a particle and an impulse is applied to the body.
///  2. After the sand tick: clear the Object pixels back to air.
pub struct RigidBody {
    pub body_handle:     RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    /// Half-extents of the bounding box in pixels.
    pub half_w: f32,
    pub half_h: f32,
    /// Pixel grid that is part of this body (for destructible bodies).
    pub pixels: Vec<MaterialInstance>,
    pub pixel_w: i32,
    pub pixel_h: i32,
}

impl RigidBody {
    pub fn new(
        phys:   &mut PhysicsWorld,
        x: f32, y: f32,
        half_w: f32, half_h: f32,
        mass:   f32,
    ) -> Self {
        let (bh, ch) = phys.add_dynamic_box(x, y, half_w, half_h, mass);
        let pw = (half_w * 2.0) as i32;
        let ph = (half_h * 2.0) as i32;
        Self {
            body_handle:     bh,
            collider_handle: ch,
            half_w,
            half_h,
            pixels: vec![MaterialInstance::air(); (pw * ph) as usize],
            pixel_w: pw,
            pixel_h: ph,
        }
    }

    /// Current pixel-space position (top-left corner of AABB).
    pub fn top_left(&self, phys: &PhysicsWorld) -> Option<(f32, f32)> {
        let (cx, cy) = phys.body_pixel_pos(self.body_handle)?;
        Some((cx - self.half_w, cy - self.half_h))
    }

    /// Step 1: stamp Object pixels into the sim world.
    /// Any sand displaced this way generates a force impulse on the body.
    pub fn pre_sim_stamp(
        &self,
        phys:  &mut PhysicsWorld,
        world: &mut SimWorld,
    ) {
        let (x0, y0) = match self.top_left(phys) {
            Some(t) => t,
            None    => return,
        };

        let object_pixel = MaterialInstance {
            id:      0,
            physics: PhysicsType::Object,
            color:   arenite_core::Color::TRANSPARENT,
            light:   [0.0; 3],
            data:    0,
        };

        let mut impulse_x = 0.0_f32;
        let mut impulse_y = 0.0_f32;

        for dy in 0..self.pixel_h {
            for dx in 0..self.pixel_w {
                let tx = (x0 + dx as f32) as i32;
                let ty = (y0 + dy as f32) as i32;
                let tp = TilePos::new(tx, ty);
                let existing = world.get_pixel(tp);

                if existing.physics.is_dynamic() {
                    // Sand hits the body — apply impulse proportional to density.
                    impulse_x += (dx as f32 - self.half_w) * 0.01;
                    impulse_y -= 0.1;
                }
                world.set_pixel(tp, object_pixel);
            }
        }

        // Apply accumulated impulse.
        if impulse_x.abs() > 0.001 || impulse_y.abs() > 0.001 {
            phys.apply_pixel_impulse(self.body_handle, impulse_x, impulse_y);
        }
    }

    /// Step 2: clear Object pixels back to air after the sim tick.
    pub fn post_sim_clear(
        &self,
        phys:  &PhysicsWorld,
        world: &mut SimWorld,
    ) {
        let (x0, y0) = match self.top_left(phys) {
            Some(t) => t,
            None    => return,
        };

        for dy in 0..self.pixel_h {
            for dx in 0..self.pixel_w {
                let tx = (x0 + dx as f32) as i32;
                let ty = (y0 + dy as f32) as i32;
                let tp = TilePos::new(tx, ty);
                let p  = world.get_pixel(tp);
                if p.physics == PhysicsType::Object {
                    world.set_pixel(tp, MaterialInstance::air());
                }
            }
        }
    }
}
