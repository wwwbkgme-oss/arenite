// Arenite Engine — game client entry point
//
// Wires together:
//  arenite-world  → WorldGenerator      (one-time procgen on startup)
//  arenite-sim    → SimWorld            (per-tick cellular automata)
//  arenite-physics→ PhysicsWorld        (per-tick Rapier2d step)
//  arenite-render → AreniteRenderer     (per-frame GPU draw)
//  arenite-net    → GameClient          (optional multiplayer)
//
// Run loop (winit event loop):
//   EventLoop::run
//     RedrawRequested:
//       1. Handle input (keyboard / mouse)
//       2. physics.pre_sim_stamp(...)      ← rigidbody → pixel bridge
//       3. sim.tick_simulation()           ← cellular automata
//       4. physics.step(dt)               ← rapier2d
//       5. physics.post_sim_clear(...)     ← clean up Object pixels
//       6. renderer.sync_chunks(&sim)      ← upload dirty textures
//       7. renderer.render()              ← draw frame

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use arenite_core::pos::{TilePos, WorldPos};
use arenite_physics::PhysicsWorld;
use arenite_render::AreniteRenderer;
use arenite_sim::{SimWorld, material::MaterialInstance, physics_type::PhysicsType};
use arenite_world::{WorldGenerator, worldgen::WorldGenConfig};
use arenite_core::Color;

/// Configurable game settings.
#[derive(Debug, serde::Deserialize)]
struct GameConfig {
    world_width:  i32,
    world_height: i32,
    world_seed:   u64,
    player_name:  String,
    #[serde(default)]
    server:       Option<String>,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            world_width:  2100,
            world_height: 600,
            world_seed:   fastrand::u64(..),
            player_name:  "Player".into(),
            server:       None,
        }
    }
}

fn load_config() -> GameConfig {
    std::fs::read_to_string("arenite.toml")
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

/// Player state.
struct Player {
    x: f32, y: f32,
    vx: f32, vy: f32,
    on_ground: bool,
    selected_mat: u16,
    brush_radius: i32,
}

impl Player {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y, vx: 0.0, vy: 0.0, on_ground: false, selected_mat: 3, brush_radius: 3 }
    }

    fn apply_gravity_and_move(&mut self, sim: &SimWorld) {
        const GRAVITY: f32 = 0.4;
        const FRICTION: f32 = 0.8;
        const MAX_FALL: f32 = 12.0;

        self.vy = (self.vy + GRAVITY).min(MAX_FALL);
        self.vx *= FRICTION;

        // Simple AABB collision with pixel world.
        let new_x = self.x + self.vx;
        let new_y = self.y + self.vy;

        // X movement.
        if !self.collide_at(sim, new_x, self.y) {
            self.x = new_x;
        } else {
            self.vx = 0.0;
        }

        // Y movement.
        if !self.collide_at(sim, self.x, new_y) {
            self.y = new_y;
            self.on_ground = false;
        } else {
            if self.vy > 0.0 { self.on_ground = true; }
            self.vy = 0.0;
        }
    }

    fn collide_at(&self, sim: &SimWorld, x: f32, y: f32) -> bool {
        let hw = 4_f32;
        let hh = 8_f32;
        for dy in [-hh, hh] {
            for dx in [-hw, 0.0, hw] {
                let tp = TilePos::new((x + dx) as i32, (y + dy) as i32);
                let p  = sim.get_pixel(tp);
                if p.physics == PhysicsType::Solid || p.physics == PhysicsType::Sand {
                    return true;
                }
            }
        }
        false
    }
}

/// The main game application.
struct AreniteApp {
    sim:      SimWorld,
    physics:  PhysicsWorld,
    player:   Player,
    renderer: Option<AreniteRenderer>,
    window:   Option<Arc<Window>>,

    // Input state.
    left: bool, right: bool, jump: bool,
    cursor_world: WorldPos,
    placing: bool, removing: bool,

    // Timing.
    last_tick: Instant,
    tick_accum: Duration,
    config: GameConfig,
}

impl AreniteApp {
    fn new(config: GameConfig) -> Self {
        let gen_cfg = WorldGenConfig {
            width:  config.world_width,
            height: config.world_height,
            seed:   config.world_seed,
            ..Default::default()
        };

        info!(
            "Generating world {}×{} seed={}",
            gen_cfg.width, gen_cfg.height, gen_cfg.seed
        );
        let sim    = WorldGenerator::new(gen_cfg).generate();
        let spawn_x = config.world_width  as f32 * 0.5;
        let spawn_y = config.world_height as f32 * 0.35;
        let player  = Player::new(spawn_x, spawn_y);

        Self {
            sim,
            physics:  PhysicsWorld::new(),
            player,
            renderer: None,
            window:   None,
            left: false, right: false, jump: false,
            cursor_world: WorldPos::new(0.0, 0.0),
            placing: false, removing: false,
            last_tick: Instant::now(),
            tick_accum: Duration::ZERO,
            config,
        }
    }

    fn update(&mut self) {
        const TICK: Duration = Duration::from_millis(16); // ~60 TPS

        let now     = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        self.last_tick  = now;
        self.tick_accum += elapsed;

        while self.tick_accum >= TICK {
            self.tick_accum -= TICK;
            self.game_tick();
        }
    }

    fn game_tick(&mut self) {
        // Player movement.
        const SPEED:      f32 = 2.0;
        const JUMP_FORCE: f32 = -8.0;

        if self.left  { self.player.vx -= SPEED; }
        if self.right { self.player.vx += SPEED; }
        if self.jump && self.player.on_ground {
            self.player.vy = JUMP_FORCE;
            self.player.on_ground = false;
        }
        self.player.apply_gravity_and_move(&self.sim);

        // Pixel painting.
        if self.placing {
            let tp  = self.cursor_world.tile();
            let mat = make_material_instance(self.player.selected_mat);
            self.sim.paint_circle(tp, self.player.brush_radius, mat);
        }
        if self.removing {
            let tp = self.cursor_world.tile();
            self.sim.paint_circle(tp, self.player.brush_radius, MaterialInstance::air());
        }

        // Tick sand simulation.
        self.sim.tick_simulation();

        // Tick physics (60 Hz, dt = 1/60 s).
        self.physics.step(1.0 / 60.0);

        // Track camera to player.
        if let Some(renderer) = &mut self.renderer {
            renderer.camera.position = glam::Vec2::new(self.player.x, self.player.y);
        }
    }
}

fn make_material_instance(selected: u16) -> MaterialInstance {
    let (physics, color) = match selected {
        0 => (PhysicsType::Sand,  Color::SAND),
        1 => (PhysicsType::Liquid,Color::WATER),
        2 => (PhysicsType::Solid, Color::STONE),
        3 => (PhysicsType::Solid, Color::DIRT),
        4 => (PhysicsType::Gas,   Color::rgb(80, 80, 80)),
        5 => (PhysicsType::Fire,  Color::rgb(255, 140, 0)),
        6 => (PhysicsType::Solid, Color::GRASS),
        7 => (PhysicsType::Sand,  Color::SNOW),
        _ => (PhysicsType::Solid, Color::STONE),
    };
    MaterialInstance { id: selected, physics, color, light: [0.0; 3], data: 50 }
}

impl ApplicationHandler for AreniteApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(
                WindowAttributes::default().with_title("Arenite Engine"),
            ).unwrap(),
        );
        self.window = Some(Arc::clone(&window));

        let renderer = pollster::block_on(AreniteRenderer::new(Arc::clone(&window)))
            .expect("Failed to create renderer");
        self.renderer = Some(renderer);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event:      WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput { event: KeyEvent { physical_key, state, .. }, .. } => {
                let pressed = state.is_pressed();
                if let PhysicalKey::Code(code) = physical_key {
                    match code {
                        KeyCode::KeyA | KeyCode::ArrowLeft  => self.left  = pressed,
                        KeyCode::KeyD | KeyCode::ArrowRight => self.right = pressed,
                        KeyCode::Space                      => self.jump  = pressed,
                        KeyCode::Escape if pressed          => event_loop.exit(),
                        // Number keys 1–8 to select material.
                        KeyCode::Digit1 if pressed => self.player.selected_mat = 0,
                        KeyCode::Digit2 if pressed => self.player.selected_mat = 1,
                        KeyCode::Digit3 if pressed => self.player.selected_mat = 2,
                        KeyCode::Digit4 if pressed => self.player.selected_mat = 3,
                        KeyCode::Digit5 if pressed => self.player.selected_mat = 4,
                        KeyCode::Digit6 if pressed => self.player.selected_mat = 5,
                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(r) = &self.renderer {
                    let w = r.camera.screen_to_world(
                        position.x as f32, position.y as f32,
                    );
                    self.cursor_world = WorldPos::new(w.x, w.y);
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state.is_pressed();
                match button {
                    MouseButton::Left  => self.placing  = pressed,
                    MouseButton::Right => self.removing = pressed,
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y)  => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if let Some(r) = &mut self.renderer {
                    r.camera.zoom_by(if scroll > 0.0 { 1.1 } else { 0.9 });
                }
            }

            WindowEvent::RedrawRequested => {
                self.update();

                if let Some(r) = &mut self.renderer {
                    r.sync_chunks(&self.sim);
                    match r.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                            let size = self.window.as_ref().unwrap().inner_size();
                            r.resize(size.width, size.height);
                        }
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let config = load_config();
    info!("Arenite Engine starting…");

    let app   = AreniteApp::new(config);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut { app })?;
    Ok(())
}
