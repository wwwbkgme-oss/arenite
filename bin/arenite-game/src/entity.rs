// entity.rs — Entity system + basic monster AI
//
// Architecture inspired by Terraria's NPC system and Starbound's
// StarNpc / StarMonster classes, adapted for the pixel-simulation world.
//
// Design:
//   Entity     — trait defining position, velocity, health, kind, and update
//   SlimeEnemy — Terraria-inspired green slime (patrol + jump + contact damage)
//   Skeleton   — Terraria-inspired skeleton archer (patrol + chase range)
//   EntityManager — owns all entities; spawns, updates, and culls dead ones
//
// Rendering is "paint-and-clear": each frame the entity stamps a colored pixel
// into the sim world for visual feedback, then marks it dirty so the renderer
// re-uploads.  A proper sprite sheet renderer is planned in a future task.

use arenite_core::pos::TilePos;
use arenite_sim::{material::MaterialInstance, physics_type::PhysicsType, SimWorld};

// ── Entity trait ─────────────────────────────────────────────────────────────

/// Functional category of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// A passive or hostile slime blob (Terraria Green Slime).
    Slime,
    /// A patrolling skeleton archer (Terraria Skeleton).
    Skeleton,
    /// A free-flying bat (Terraria Cave Bat).
    Bat,
    /// A dropped item waiting to be picked up.
    ItemDrop,
}

/// Core entity interface.
pub trait Entity: Send {
    fn kind(&self) -> EntityKind;

    fn position(&self) -> (f32, f32);
    fn velocity(&self) -> (f32, f32);
    fn health(&self) -> i32;
    fn max_health(&self) -> i32;
    fn is_alive(&self) -> bool {
        self.health() > 0
    }

    /// Width × height of the AABB hitbox in pixels.
    fn hitbox(&self) -> (f32, f32);

    /// Tick the entity's AI and physics.
    ///
    /// `player_x / player_y` are the player's current world-pixel coordinates,
    /// used by chase/aggro logic.
    fn tick(&mut self, sim: &SimWorld, player_x: f32, player_y: f32);

    /// Apply `amount` points of damage to this entity.
    fn take_damage(&mut self, amount: i32);

    /// Return the colour used for the "paint-and-clear" visual pixel.
    fn pixel_color(&self) -> arenite_core::Color;

    /// True if the entity overlaps the given world-pixel rectangle.
    fn overlaps(&self, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
        let (ex, ey) = self.position();
        let (ew, eh) = self.hitbox();
        ex < rx + rw && ex + ew > rx && ey < ry + rh && ey + eh > ry
    }

    /// Return contact-damage dealt to the player per tick (0 = harmless).
    fn contact_damage(&self) -> i32 {
        0
    }
}

// ── Physics helpers ───────────────────────────────────────────────────────────

/// Check whether an AABB at (x, y, w, h) overlaps a solid/sand pixel.
fn collide_at(sim: &SimWorld, x: f32, y: f32, w: f32, h: f32) -> bool {
    let check_corners = [
        (x, y),
        (x + w - 1.0, y),
        (x, y + h - 1.0),
        (x + w - 1.0, y + h - 1.0),
    ];
    for (cx, cy) in check_corners {
        let tp = TilePos::new(cx as i32, cy as i32);
        let p = sim.get_pixel(tp);
        if matches!(p.physics, PhysicsType::Solid | PhysicsType::Sand) {
            return true;
        }
    }
    false
}

/// Integrate entity position against the pixel world.
/// Returns (new_x, new_y, on_ground).
fn integrate(sim: &SimWorld, x: f32, y: f32, vx: f32, vy: f32, w: f32, h: f32) -> (f32, f32, bool) {
    // X axis
    let nx = x + vx;
    let actual_x = if collide_at(sim, nx, y, w, h) { x } else { nx };

    // Y axis
    let ny = y + vy;
    let (actual_y, on_ground) = if collide_at(sim, actual_x, ny, w, h) {
        (y, vy > 0.0)
    } else {
        (ny, false)
    };

    (actual_x, actual_y, on_ground)
}

// ── SlimeEnemy ────────────────────────────────────────────────────────────────

/// A simple green slime that patrols left/right and jumps when blocked.
/// Deals contact damage.  Inspired by Terraria's Green Slime.
pub struct SlimeEnemy {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hp: i32,
    on_ground: bool,
    dir: f32,       // +1 or -1
    dir_timer: u32, // ticks until direction reversal
    jump_cooldown: u32,
    aggro_range: f32, // pixels — chase player within this range
}

impl SlimeEnemy {
    const W: f32 = 10.0;
    const H: f32 = 10.0;
    const SPEED: f32 = 0.8;
    const JUMP_FORCE: f32 = -7.5;
    const GRAVITY: f32 = 0.4;
    const MAX_FALL: f32 = 12.0;
    const HP: i32 = 40;

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: Self::HP,
            on_ground: false,
            dir: 1.0,
            dir_timer: fastrand::u32(60..180),
            jump_cooldown: 0,
            aggro_range: 120.0,
        }
    }
}

impl Entity for SlimeEnemy {
    fn kind(&self) -> EntityKind {
        EntityKind::Slime
    }
    fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }
    fn velocity(&self) -> (f32, f32) {
        (self.vx, self.vy)
    }
    fn health(&self) -> i32 {
        self.hp
    }
    fn max_health(&self) -> i32 {
        Self::HP
    }
    fn hitbox(&self) -> (f32, f32) {
        (Self::W, Self::H)
    }
    fn contact_damage(&self) -> i32 {
        5
    }
    fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
    }
    fn pixel_color(&self) -> arenite_core::Color {
        // Colour shifts from green → red as health drops.
        let ratio = (self.hp as f32 / Self::HP as f32).clamp(0.0, 1.0);
        arenite_core::Color::new(
            ((1.0 - ratio) * 200.0) as u8,
            (ratio * 180.0) as u8,
            20,
            220,
        )
    }

    fn tick(&mut self, sim: &SimWorld, player_x: f32, player_y: f32) {
        // ── AI ────────────────────────────────────────────────────────────
        let dx_to_player = player_x - self.x;
        let dy_to_player = (player_y - self.y).abs();
        let dist = dx_to_player.abs().hypot(dy_to_player);

        if dist < self.aggro_range {
            // Chase player.
            self.dir = dx_to_player.signum();
        } else {
            // Patrol.
            if self.dir_timer == 0 {
                self.dir = -self.dir;
                self.dir_timer = fastrand::u32(60..180);
            } else {
                self.dir_timer -= 1;
            }
        }

        // Horizontal movement.
        self.vx = self.dir * Self::SPEED;

        // Jump when blocked horizontally and on ground.
        if self.on_ground && self.jump_cooldown == 0 {
            let peek_x = self.x + self.vx * 4.0;
            if collide_at(sim, peek_x, self.y, Self::W, Self::H) {
                self.vy = Self::JUMP_FORCE;
                self.on_ground = false;
                self.jump_cooldown = 30;
                self.dir = -self.dir; // bounce direction
            }
            // Occasional random jump.
            if fastrand::u8(..) < 3 {
                self.vy = Self::JUMP_FORCE * 0.8;
                self.on_ground = false;
                self.jump_cooldown = 40;
            }
        }
        if self.jump_cooldown > 0 {
            self.jump_cooldown -= 1;
        }

        // ── Physics ───────────────────────────────────────────────────────
        self.vy = (self.vy + Self::GRAVITY).min(Self::MAX_FALL);

        let (nx, ny, og) = integrate(sim, self.x, self.y, self.vx, self.vy, Self::W, Self::H);
        self.x = nx;
        self.y = ny;
        self.on_ground = og;
        if og {
            self.vy = 0.0;
        }
        // Horizontal friction.
        if self.on_ground {
            self.vx *= 0.8;
        }
    }
}

// ── Bat ───────────────────────────────────────────────────────────────────────

/// A cave bat that flutters toward the player in a wavy pattern.
/// Inspired by Terraria's Cave Bat.
pub struct BatEnemy {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hp: i32,
    phase: f32, // for sinusoidal flight
}

impl BatEnemy {
    const W: f32 = 8.0;
    const H: f32 = 6.0;
    const SPEED: f32 = 1.2;
    const HP: i32 = 20;

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: Self::HP,
            phase: fastrand::f32() * std::f32::consts::TAU,
        }
    }
}

impl Entity for BatEnemy {
    fn kind(&self) -> EntityKind {
        EntityKind::Bat
    }
    fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }
    fn velocity(&self) -> (f32, f32) {
        (self.vx, self.vy)
    }
    fn health(&self) -> i32 {
        self.hp
    }
    fn max_health(&self) -> i32 {
        Self::HP
    }
    fn hitbox(&self) -> (f32, f32) {
        (Self::W, Self::H)
    }
    fn contact_damage(&self) -> i32 {
        3
    }
    fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
    }
    fn pixel_color(&self) -> arenite_core::Color {
        arenite_core::Color::new(60, 20, 80, 200)
    }

    fn tick(&mut self, _sim: &SimWorld, player_x: f32, player_y: f32) {
        self.phase += 0.12;
        // Fly toward player with a sinusoidal vertical oscillation.
        let dx = player_x - self.x;
        let dy = player_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
        self.vx = (dx / dist) * Self::SPEED;
        self.vy = (dy / dist) * Self::SPEED + self.phase.sin() * 0.8;
        self.x += self.vx;
        self.y += self.vy;
    }
}

// ── EntityManager ─────────────────────────────────────────────────────────────

/// Manages all live entities in the world.
pub struct EntityManager {
    entities: Vec<Box<dyn Entity>>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn spawn(&mut self, entity: Box<dyn Entity>) {
        self.entities.push(entity);
    }

    /// Tick all entities and remove dead ones.
    /// Returns the total contact damage dealt to the player this tick.
    pub fn update(
        &mut self,
        sim: &SimWorld,
        player_x: f32,
        player_y: f32,
        player_w: f32,
        player_h: f32,
    ) -> i32 {
        let mut contact_dmg = 0_i32;

        for e in self.entities.iter_mut() {
            e.tick(sim, player_x, player_y);

            // Check if this entity overlaps the player's AABB.
            if e.overlaps(player_x, player_y, player_w, player_h) {
                contact_dmg += e.contact_damage();
            }
        }

        // Remove dead entities.
        self.entities.retain(|e| e.is_alive());

        contact_dmg
    }

    /// Stamp each entity as a colored pixel into the sim world so the renderer
    /// can show them.  Call BEFORE sync_chunks, after the sim tick.
    /// The pixel at the entity centre is overwritten every frame; since it's
    /// marked dirty the GPU texture is refreshed automatically.
    pub fn stamp_pixels(&self, sim: &mut SimWorld) {
        for e in &self.entities {
            let (x, y) = e.position();
            let (w, h) = e.hitbox();
            let cx = (x + w * 0.5) as i32;
            let cy = (y + h * 0.5) as i32;
            let tp = TilePos::new(cx, cy);
            // Only stamp into air — don't overwrite terrain.
            if sim.get_pixel(tp).is_air() {
                let pixel = MaterialInstance {
                    id: 0,
                    physics: PhysicsType::Object,
                    color: e.pixel_color(),
                    light: [0.0; 3],
                    data: 0,
                };
                sim.set_pixel(tp, pixel);
            }
        }
    }

    /// Clear entity pixels stamped in the previous frame.
    /// Call AFTER sync_chunks / renderer upload, before the next sim tick.
    pub fn clear_pixels(&self, sim: &mut SimWorld) {
        for e in &self.entities {
            let (x, y) = e.position();
            let (w, h) = e.hitbox();
            let cx = (x + w * 0.5) as i32;
            let cy = (y + h * 0.5) as i32;
            let tp = TilePos::new(cx, cy);
            let p = sim.get_pixel(tp);
            if p.physics == PhysicsType::Object {
                sim.set_pixel(tp, MaterialInstance::air());
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Spawn a batch of slimes scattered around a centre point.
    pub fn spawn_slimes_near(&mut self, cx: f32, cy: f32, count: usize, radius: f32) {
        for _ in 0..count {
            let angle = fastrand::f32() * std::f32::consts::TAU;
            let r = fastrand::f32() * radius;
            let sx = cx + angle.cos() * r;
            let sy = cy + angle.sin() * r;
            self.spawn(Box::new(SlimeEnemy::new(sx, sy)));
        }
    }

    /// Spawn a batch of bats scattered underground near a position.
    pub fn spawn_bats_near(&mut self, cx: f32, cy: f32, count: usize, radius: f32) {
        for _ in 0..count {
            let angle = fastrand::f32() * std::f32::consts::TAU;
            let r = fastrand::f32() * radius;
            self.spawn(Box::new(BatEnemy::new(
                cx + angle.cos() * r,
                cy + angle.sin() * r,
            )));
        }
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}
