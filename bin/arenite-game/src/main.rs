// Arenite Engine — game client
//
// Gray-window fixes applied (T-015..T-019):
//  T-015 Camera set to spawn pos inside resumed() immediately after renderer creation
//  T-016 Vertex buffers collected before render pass in renderer.rs (persistent vbuf)
//  T-017 Default world 600×200 tiles — generates in < 0.5 s on most hardware
//  T-018 World gen runs in a std::thread; game loop shows loading state while pending
//  T-019 Spawn scanner: finds first solid tile above the surface centre column

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use arenite_core::pos::{TilePos, WorldPos};
use arenite_physics::PhysicsWorld;
use arenite_render::AreniteRenderer;
use arenite_sim::{SimWorld, save_world, load_world, material::MaterialInstance, physics_type::PhysicsType};
use arenite_world::{WorldGenerator, worldgen::WorldGenConfig};
use arenite_core::Color;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct GameConfig {
    world_width:  i32,
    world_height: i32,
    world_seed:   u64,
    #[allow(dead_code)]
    player_name:  String,
    #[serde(default)]
    #[allow(dead_code)]
    server: Option<String>,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            // T-017: small default — generates in < 0.5 s; set larger in arenite.toml
            world_width:  600,
            world_height: 200,
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

// ── Material palette ──────────────────────────────────────────────────────────

const PALETTE: &[(PhysicsType, Color, &str)] = &[
    (PhysicsType::Sand,  Color::SAND,              "Sand"),
    (PhysicsType::Liquid,Color::WATER,             "Water"),
    (PhysicsType::Solid, Color::STONE,             "Stone"),
    (PhysicsType::Solid, Color::DIRT,              "Dirt"),
    (PhysicsType::Solid, Color::GRASS,             "Grass"),
    (PhysicsType::Sand,  Color::SNOW,              "Snow"),
    (PhysicsType::Gas,   Color { r:80,g:80,b:80,a:160 },  "Smoke"),
    (PhysicsType::Liquid,Color::LAVA,              "Lava"),
];

fn make_mat(slot: usize) -> MaterialInstance {
    let (physics, color, _) = PALETTE[slot % PALETTE.len()];
    MaterialInstance { id: slot as u16, physics, color, light: [0.0; 3], data: 50 }
}

// ── Player ────────────────────────────────────────────────────────────────────

struct Player {
    x: f32, y: f32,
    vx: f32, vy: f32,
    on_ground:    bool,
    selected_mat: usize,
    brush_radius: i32,
}

impl Player {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y, vx: 0.0, vy: 0.0, on_ground: false, selected_mat: 0, brush_radius: 3 }
    }

    fn collide_at(&self, sim: &SimWorld, x: f32, y: f32) -> bool {
        for dy in &[-7_f32, 7.0] {
            for dx in &[-4_f32, 0.0, 4.0] {
                let tp = TilePos::new((x + dx) as i32, (y + dy) as i32);
                let p  = sim.get_pixel(tp);
                if matches!(p.physics, PhysicsType::Solid | PhysicsType::Sand) {
                    return true;
                }
            }
        }
        false
    }

    fn tick(&mut self, sim: &SimWorld, left: bool, right: bool, jump: bool) {
        const SPEED:      f32 = 2.5;
        const JUMP_FORCE: f32 = -9.0;
        const GRAVITY:    f32 = 0.45;
        const FRICTION:   f32 = 0.78;
        const MAX_FALL:   f32 = 14.0;

        if left  { self.vx -= SPEED; }
        if right { self.vx += SPEED; }
        if jump && self.on_ground {
            self.vy = JUMP_FORCE;
            self.on_ground = false;
        }

        self.vy = (self.vy + GRAVITY).min(MAX_FALL);
        self.vx *= FRICTION;

        let nx = self.x + self.vx;
        if !self.collide_at(sim, nx, self.y) { self.x = nx; } else { self.vx = 0.0; }

        let ny = self.y + self.vy;
        if !self.collide_at(sim, self.x, ny) {
            self.y = ny;
            self.on_ground = false;
        } else {
            if self.vy > 0.0 { self.on_ground = true; }
            self.vy = 0.0;
        }
    }
}

// ── T-019: find spawn above first solid tile in centre column ─────────────────

fn find_spawn(sim: &SimWorld, width: i32, height: i32) -> (f32, f32) {
    let cx = width / 2;
    for y in 1..height {
        let tp = TilePos::new(cx, y);
        if sim.get_pixel(tp).physics == PhysicsType::Solid {
            // Stand 3 px above the surface.
            return (cx as f32, (y - 3) as f32);
        }
    }
    // Fallback: upper third
    (cx as f32, (height as f32 * 0.25))
}

// ── World state (T-018: background gen) ──────────────────────────────────────

enum WorldState {
    /// World is being generated on a background thread.
    Loading(thread::JoinHandle<SimWorld>),
    /// World is ready.
    Ready(SimWorld),
    /// Placeholder while we take ownership.
    Empty,
}

// ── App ───────────────────────────────────────────────────────────────────────

struct AreniteApp {
    world:        WorldState,
    physics:      PhysicsWorld,
    player:       Player,
    renderer:     Option<AreniteRenderer>,
    window:       Option<Arc<Window>>,
    world_width:  i32,
    world_height: i32,

    // Input
    left: bool, right: bool, jump: bool,
    cursor_world: WorldPos,
    placing: bool, removing: bool,

    // Timing
    last_tick:  Instant,
    tick_accum: Duration,
    frame_count: u64,
    fps_timer:   Instant,

    #[allow(dead_code)]
    config: GameConfig,
}

impl AreniteApp {
    fn new(config: GameConfig) -> Self {
        let cfg = WorldGenConfig {
            width:  config.world_width,
            height: config.world_height,
            seed:   config.world_seed,
            ..Default::default()
        };
        let w = config.world_width;
        let h = config.world_height;

        info!("Starting background world gen {}×{} seed={}", cfg.width, cfg.height, cfg.seed);

        // T-018: spawn generation on a thread so the event loop starts immediately.
        let handle = thread::spawn(move || {
            let sim = WorldGenerator::new(cfg).generate();
            info!("World gen complete ({} chunks)", sim.loaded_chunk_count());
            sim
        });

        Self {
            world:       WorldState::Loading(handle),
            physics:     PhysicsWorld::new(),
            player:      Player::new(w as f32 * 0.5, 10.0),  // temp position; corrected on ready
            renderer:    None,
            window:      None,
            world_width:  w,
            world_height: h,
            left: false, right: false, jump: false,
            cursor_world: WorldPos::new(0.0, 0.0),
            placing: false, removing: false,
            last_tick:   Instant::now(),
            tick_accum:  Duration::ZERO,
            frame_count: 0,
            fps_timer:   Instant::now(),
            config,
        }
    }

    /// Poll the background gen thread; returns true if world just became ready.
    fn poll_world(&mut self) -> bool {
        let ready = matches!(&self.world, WorldState::Loading(h) if h.is_finished());
        if ready {
            let old = std::mem::replace(&mut self.world, WorldState::Empty);
            if let WorldState::Loading(handle) = old {
                if let Ok(sim) = handle.join() {
                    // T-019: correct spawn position now that world is generated.
                    let (sx, sy) = find_spawn(&sim, self.world_width, self.world_height);
                    self.player = Player::new(sx, sy);
                    info!("Player spawn: ({:.0}, {:.0})", sx, sy);

                    // T-015: set camera immediately so frame 1 shows the right area.
                    if let Some(r) = &mut self.renderer {
                        r.camera.position = glam::Vec2::new(sx, sy);
                    }

                    self.world = WorldState::Ready(sim);
                    return true;
                }
            }
        }
        false
    }

    fn update(&mut self) {
        self.poll_world();

        const TICK: Duration = Duration::from_millis(16); // 60 TPS
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
        let WorldState::Ready(sim) = &mut self.world else { return; };

        // Pixel painting.
        if self.placing {
            let tp  = self.cursor_world.tile();
            let mat = make_mat(self.player.selected_mat);
            sim.paint_circle(tp, self.player.brush_radius, mat);
        }
        if self.removing {
            let tp = self.cursor_world.tile();
            sim.paint_circle(tp, self.player.brush_radius, MaterialInstance::air());
        }

        // Player physics + movement.
        self.player.tick(sim, self.left, self.right, self.jump);

        // Cellular automata tick.
        sim.tick_simulation();

        // Rapier2d step.
        self.physics.step(1.0 / 60.0);

        // T-015: keep camera locked to player every tick.
        if let Some(r) = &mut self.renderer {
            r.camera.position = glam::Vec2::new(self.player.x, self.player.y);
        }
    }

    fn update_window_title(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            if let Some(w) = &self.window {
                let mat_name = PALETTE[self.player.selected_mat % PALETTE.len()].2;
                let title = format!(
                    "Arenite  |  {fps:.0} fps  |  [{mat_name}]  |  brush:{br}  |  {x:.0},{y:.0}",
                    fps = fps,
                    mat_name = mat_name,
                    br = self.player.brush_radius,
                    x = self.player.x,
                    y = self.player.y,
                );
                w.set_title(&title);
            }
            self.frame_count = 0;
            self.fps_timer   = Instant::now();
        }
    }
}

impl ApplicationHandler for AreniteApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("Arenite Engine — Loading…"))
                .expect("Failed to create window"),
        );
        self.window = Some(Arc::clone(&window));

        let renderer = pollster::block_on(AreniteRenderer::new(Arc::clone(&window)))
            .expect("Failed to initialise GPU renderer");

        // T-015: set camera to current player position before the first draw.
        let mut r = renderer;
        r.camera.position = glam::Vec2::new(self.player.x, self.player.y);
        r.camera.zoom = 2.0; // pixel-doubled for crisp 1px art

        self.renderer = Some(r);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
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
                        KeyCode::Space | KeyCode::ArrowUp   => self.jump  = pressed,
                        KeyCode::Escape if pressed          => event_loop.exit(),

                        // Material slots
                        KeyCode::Digit1 if pressed => self.player.selected_mat = 0,
                        KeyCode::Digit2 if pressed => self.player.selected_mat = 1,
                        KeyCode::Digit3 if pressed => self.player.selected_mat = 2,
                        KeyCode::Digit4 if pressed => self.player.selected_mat = 3,
                        KeyCode::Digit5 if pressed => self.player.selected_mat = 4,
                        KeyCode::Digit6 if pressed => self.player.selected_mat = 5,
                        KeyCode::Digit7 if pressed => self.player.selected_mat = 6,
                        KeyCode::Digit8 if pressed => self.player.selected_mat = 7,

                        // T-031: brush size
                        KeyCode::BracketLeft  if pressed =>
                            self.player.brush_radius = (self.player.brush_radius - 1).max(1),
                        KeyCode::BracketRight if pressed =>
                            self.player.brush_radius = (self.player.brush_radius + 1).min(32),

                        // T-024: save / load / pause
                        KeyCode::KeyS if pressed => {
                            if let WorldState::Ready(sim) = &self.world {
                                let dir = std::path::Path::new("saves");
                                match save_world(sim, dir, "default") {
                                    Ok(_)  => info!("World saved to saves/default/"),
                                    Err(e) => log::error!("Save failed: {e}"),
                                }
                            }
                        }
                        KeyCode::KeyL if pressed => {
                            let dir = std::path::Path::new("saves");
                            match load_world(dir, "default") {
                                Ok(sim) => {
                                    let (sx, sy) = find_spawn(&sim, self.world_width, self.world_height);
                                    self.player = Player::new(sx, sy);
                                    if let Some(r) = &mut self.renderer {
                                        r.camera.position = glam::Vec2::new(sx, sy);
                                        r.sync_chunks(&sim);
                                    }
                                    self.world = WorldState::Ready(sim);
                                    info!("World loaded from saves/default/");
                                }
                                Err(e) => log::error!("Load failed: {e}"),
                            }
                        }

                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(r) = &self.renderer {
                    let w = r.camera.screen_to_world(position.x as f32, position.y as f32);
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
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p)   => p.y as f32 / 40.0,
                };
                if let Some(r) = &mut self.renderer {
                    r.camera.zoom_by(if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 });
                }
            }

            WindowEvent::RedrawRequested => {
                self.update();
                self.update_window_title();

                if let Some(r) = &mut self.renderer {
                    if let WorldState::Ready(sim) = &self.world {
                        r.sync_chunks(sim);
                    }
                    match r.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                            let s = self.window.as_ref().unwrap().inner_size();
                            r.resize(s.width, s.height);
                        }
                        Err(e) => log::error!("Render error: {e:?}"),
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

    info!("Arenite Engine");
    let config     = load_config();
    let app        = AreniteApp::new(config);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut { app })?;
    Ok(())
}
