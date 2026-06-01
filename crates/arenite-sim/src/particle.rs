use crate::material::MaterialInstance;
use crate::physics_type::PhysicsType;
use arenite_core::Color;

/// A free-flying pixel particle, not bound to the chunk grid.
/// Used for displaced pixels, explosions, sparks, etc.
#[derive(Clone, Debug)]
pub struct Particle {
    pub material: MaterialInstance,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Remaining lifetime in simulation ticks.
    pub lifetime: u32,
    /// Whether gravity applies.
    pub affected_by_gravity: bool,
}

impl Particle {
    pub fn new(material: MaterialInstance, x: f32, y: f32, vx: f32, vy: f32) -> Self {
        let lifetime = match material.physics {
            PhysicsType::Fire => fastrand::u32(10..40),
            PhysicsType::Gas => fastrand::u32(30..120),
            PhysicsType::Sand => fastrand::u32(60..200),
            _ => fastrand::u32(20..80),
        };
        Self {
            material,
            x,
            y,
            vx,
            vy,
            lifetime,
            affected_by_gravity: !matches!(material.physics, PhysicsType::Gas | PhysicsType::Fire),
        }
    }

    /// Advance the particle by one tick.
    /// Returns `false` when the particle should be removed.
    pub fn tick(&mut self) -> bool {
        const GRAVITY: f32 = 0.15;
        const DRAG: f32 = 0.98;

        if self.affected_by_gravity {
            self.vy += GRAVITY;
        } else {
            self.vy -= GRAVITY * 0.2; // gases rise
        }
        self.vx *= DRAG;
        self.vy *= DRAG;

        self.x += self.vx;
        self.y += self.vy;

        if self.lifetime == 0 {
            return false;
        }
        self.lifetime -= 1;
        true
    }

    /// Tile position of this particle (floor).
    pub fn tile_x(&self) -> i32 {
        self.x.floor() as i32
    }
    pub fn tile_y(&self) -> i32 {
        self.y.floor() as i32
    }

    /// Visual colour with alpha fade near end of life.
    pub fn display_color(&self) -> Color {
        let alpha = ((self.lifetime as f32 / 60.0).min(1.0) * 255.0) as u8;
        Color {
            a: alpha,
            ..self.material.color
        }
    }
}
