// Arenite Engine — dedicated headless server
//
// Runs the world simulation and network server with no rendering.
// Players connect via TCP (default port 25565).
//
// Architecture:
//   main thread   → sim tick loop (fixed 20 TPS)
//   tokio runtime → TCP accept loop + per-client tasks

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::Result;
use log::info;

use arenite_net::GameServer;
use arenite_sim::SimWorld;
use arenite_world::{WorldGenerator, worldgen::WorldGenConfig};

#[derive(Debug, serde::Deserialize)]
struct ServerConfig {
    bind:         String,
    world_width:  i32,
    world_height: i32,
    world_seed:   u64,
    tps:          u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind:         "0.0.0.0:25565".into(),
            world_width:  4200,
            world_height: 1200,
            world_seed:   fastrand::u64(..),
            tps:          20,
        }
    }
}

fn load_config() -> ServerConfig {
    std::fs::read_to_string("arenite-server.toml")
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let config = load_config();
    info!(
        "Arenite Server — world {}×{} seed={} tps={}",
        config.world_width, config.world_height, config.world_seed, config.tps
    );

    // Generate the world.
    let gen_cfg = WorldGenConfig {
        width:  config.world_width,
        height: config.world_height,
        seed:   config.world_seed,
        ..Default::default()
    };
    let mut sim = WorldGenerator::new(gen_cfg).generate();
    info!("World generated. {} chunks loaded.", sim.loaded_chunk_count());

    // Start the network server on a separate tokio task.
    let addr: SocketAddr = config.bind.parse()?;
    let net_server = GameServer::new(addr);

    let seed   = config.world_seed;
    let width  = config.world_width;
    let height = config.world_height;

    tokio::spawn(async move {
        if let Err(e) = net_server.run(seed, width, height).await {
            log::error!("Network server error: {}", e);
        }
    });

    // Sim tick loop — runs at `config.tps` ticks per second.
    let tick_duration = Duration::from_secs_f64(1.0 / config.tps as f64);
    let mut last_tick = Instant::now();
    let mut tick_count: u64 = 0;

    loop {
        let now     = Instant::now();
        let elapsed = now.duration_since(last_tick);

        if elapsed >= tick_duration {
            last_tick = now;
            sim.tick_simulation();
            tick_count += 1;

            // Log stats every 10 seconds.
            if tick_count % (config.tps as u64 * 10) == 0 {
                info!(
                    "Tick {}  active_chunks={}  particles={}",
                    tick_count,
                    sim.active_chunk_count(),
                    sim.particles.len(),
                );
            }
        } else {
            // Yield to tokio tasks (network) for the remaining slice.
            tokio::time::sleep(tick_duration - elapsed).await;
        }
    }
}
