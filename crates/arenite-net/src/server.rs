use log::{info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use crate::codec::{read_message, write_message};
use crate::message::{NetMessage, PROTOCOL_VERSION};

/// Per-connected-client state held by the server.
#[derive(Debug)]
pub struct ConnectedPlayer {
    pub id: u32,
    pub name: String,
    pub addr: SocketAddr,
    /// Send a message to this player's write task.
    pub tx: mpsc::UnboundedSender<NetMessage>,
}

pub type ServerPlayerMap = Arc<Mutex<HashMap<u32, ConnectedPlayer>>>;

/// Headless game server: accepts TCP connections and routes messages.
pub struct GameServer {
    pub addr: SocketAddr,
    pub players: ServerPlayerMap,
    next_id: Arc<Mutex<u32>>,
}

impl GameServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            players: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Run the server accept loop (spawn with `tokio::spawn` or `#[tokio::main]`).
    pub async fn run(
        self,
        world_seed: u64,
        world_width: i32,
        world_height: i32,
    ) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("Arenite server listening on {}", self.addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("Client connected from {}", addr);

            let players = Arc::clone(&self.players);
            let next_id = Arc::clone(&self.next_id);

            tokio::spawn(async move {
                if let Err(e) = handle_client(
                    stream,
                    addr,
                    players,
                    next_id,
                    world_seed,
                    world_width,
                    world_height,
                )
                .await
                {
                    warn!("Client {} error: {}", addr, e);
                }
            });
        }
    }
}

async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    players: ServerPlayerMap,
    next_id_ref: Arc<Mutex<u32>>,
    world_seed: u64,
    world_width: i32,
    world_height: i32,
) -> anyhow::Result<()> {
    // ── Handshake ──────────────────────────────────────────────────────────
    let player_name = match read_message(&mut stream).await? {
        NetMessage::Hello {
            protocol_version,
            player_name,
        } if protocol_version == PROTOCOL_VERSION => player_name,
        NetMessage::Hello {
            protocol_version, ..
        } => {
            write_message(
                &mut stream,
                &NetMessage::Reject {
                    reason: format!(
                        "Protocol mismatch: client={} server={}",
                        protocol_version, PROTOCOL_VERSION
                    ),
                },
            )
            .await?;
            return Ok(());
        }
        other => {
            write_message(
                &mut stream,
                &NetMessage::Reject {
                    reason: format!("Expected Hello, got {:?}", other),
                },
            )
            .await?;
            return Ok(());
        }
    };

    let player_id = {
        let mut id = next_id_ref.lock().unwrap();
        let cur = *id;
        *id += 1;
        cur
    };

    write_message(
        &mut stream,
        &NetMessage::Welcome {
            player_id,
            world_seed,
            world_width,
            world_height,
            spawn_x: world_width as f32 * 0.5,
            spawn_y: world_height as f32 * 0.35,
        },
    )
    .await?;

    info!("Player '{}' joined (id={})", player_name, player_id);

    // Notify existing players.
    broadcast_except(
        &players,
        player_id,
        NetMessage::PlayerJoined {
            player_id,
            name: player_name.clone(),
        },
    );

    // ── Per-client channels ────────────────────────────────────────────────
    let (tx, mut rx) = mpsc::unbounded_channel::<NetMessage>();
    {
        players.lock().unwrap().insert(
            player_id,
            ConnectedPlayer {
                id: player_id,
                name: player_name.clone(),
                addr,
                tx,
            },
        );
    }

    // Split the TcpStream into independent halves.
    let (mut reader, mut writer) = stream.into_split();

    let players_clone = Arc::clone(&players);

    // ── Write task ─────────────────────────────────────────────────────────
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match bincode::serialize(&msg) {
                Ok(bytes) => {
                    let len = bytes.len() as u32;
                    if writer.write_all(&len.to_le_bytes()).await.is_err() {
                        break;
                    }
                    if writer.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Err(e) => warn!("Serialise error: {}", e),
            }
        }
    });

    // ── Read loop ──────────────────────────────────────────────────────────
    loop {
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(_) => break,
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        if reader.read_exact(&mut buf).await.is_err() {
            break;
        }

        match bincode::deserialize::<NetMessage>(&buf) {
            Ok(NetMessage::Disconnect) => break,
            Ok(msg) => {
                // TODO: route input/chat messages to game logic
                let _ = msg;
            }
            Err(e) => {
                warn!("Deserialise error from {}: {}", addr, e);
            }
        }
    }

    // ── Cleanup ────────────────────────────────────────────────────────────
    write_task.abort();
    players_clone.lock().unwrap().remove(&player_id);
    broadcast_except(
        &players_clone,
        player_id,
        NetMessage::PlayerLeft { player_id },
    );
    info!("Player '{}' (id={}) disconnected", player_name, player_id);
    Ok(())
}

fn broadcast_except(players: &ServerPlayerMap, skip_id: u32, msg: NetMessage) {
    let ps = players.lock().unwrap();
    for p in ps.values() {
        if p.id != skip_id {
            let _ = p.tx.send(msg.clone());
        }
    }
}
