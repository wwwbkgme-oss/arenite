use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use log::{info, warn};

use crate::codec::{read_message, write_message};
use crate::message::{NetMessage, PROTOCOL_VERSION};

/// Per-connected-client state held by the server.
#[derive(Debug)]
pub struct ConnectedPlayer {
    pub id:   u32,
    pub name: String,
    pub addr: SocketAddr,
    pub tx:   mpsc::UnboundedSender<NetMessage>,
}

pub type ServerPlayerMap = Arc<Mutex<HashMap<u32, ConnectedPlayer>>>;

/// Headless game server: accepts TCP connections and routes messages.
pub struct GameServer {
    pub addr:    SocketAddr,
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

    /// Run the server event loop (call with tokio::spawn or #[tokio::main]).
    pub async fn run(
        self,
        world_seed:   u64,
        world_width:  i32,
        world_height: i32,
    ) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("Arenite server listening on {}", self.addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("Client connected from {}", addr);

            let players_ref  = Arc::clone(&self.players);
            let next_id_ref  = Arc::clone(&self.next_id);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(
                    stream, addr,
                    players_ref, next_id_ref,
                    world_seed, world_width, world_height,
                ).await {
                    warn!("Client {} error: {}", addr, e);
                }
            });
        }
    }

    async fn handle_client(
        mut stream:     TcpStream,
        addr:           SocketAddr,
        players:        ServerPlayerMap,
        next_id_ref:    Arc<Mutex<u32>>,
        world_seed:     u64,
        world_width:    i32,
        world_height:   i32,
    ) -> anyhow::Result<()> {
        // Expect Hello.
        let hello = read_message(&mut stream).await?;
        let player_name = match hello {
            NetMessage::Hello { protocol_version, player_name }
                if protocol_version == PROTOCOL_VERSION => player_name,
            NetMessage::Hello { protocol_version, .. } => {
                write_message(&mut stream, &NetMessage::Reject {
                    reason: format!(
                        "Protocol version mismatch: client={} server={}",
                        protocol_version, PROTOCOL_VERSION
                    ),
                }).await?;
                return Ok(());
            }
            _ => {
                write_message(&mut stream, &NetMessage::Reject {
                    reason: "Expected Hello".into()
                }).await?;
                return Ok(());
            }
        };

        // Assign player ID.
        let player_id = {
            let mut id = next_id_ref.lock().unwrap();
            let cur = *id;
            *id += 1;
            cur
        };

        // Send Welcome.
        write_message(&mut stream, &NetMessage::Welcome {
            player_id,
            world_seed,
            world_width,
            world_height,
            spawn_x: world_width  as f32 * 0.5,
            spawn_y: world_height as f32 * 0.35,
        }).await?;

        info!("Player '{}' joined as id={}", player_name, player_id);

        // Notify existing players.
        {
            let ps = players.lock().unwrap();
            for p in ps.values() {
                let _ = p.tx.send(NetMessage::PlayerJoined {
                    player_id,
                    name: player_name.clone(),
                });
            }
        }

        // Register player.
        let (tx, mut rx) = mpsc::unbounded_channel::<NetMessage>();
        {
            let mut ps = players.lock().unwrap();
            ps.insert(player_id, ConnectedPlayer {
                id:   player_id,
                name: player_name.clone(),
                addr,
                tx,
            });
        }

        // Main message loop.
        let (reader_half, writer_half) = stream.into_split();
        let mut reader = tokio::net::tcp::OwnedReadHalf::from(reader_half);
        let mut writer = tokio::net::tcp::OwnedWriteHalf::from(writer_half);

        let players_clone = Arc::clone(&players);
        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let bytes = bincode::serialize(&msg).unwrap_or_default();
                let len   = bytes.len() as u32;
                if writer.write_all(&len.to_le_bytes()).await.is_err() { break; }
                if writer.write_all(&bytes).await.is_err() { break; }
            }
        });

        // Read loop.
        let mut reader_stream = TcpStream::from_std(
            std::net::TcpStream::connect(addr)?
        )?;

        loop {
            match read_message(&mut reader_stream).await {
                Ok(NetMessage::Disconnect) | Err(_) => break,
                Ok(msg) => {
                    // In a full implementation: route to game logic.
                    let _ = msg;
                }
            }
        }

        write_task.abort();
        {
            let mut ps = players_clone.lock().unwrap();
            ps.remove(&player_id);
            for p in ps.values() {
                let _ = p.tx.send(NetMessage::PlayerLeft { player_id });
            }
        }
        info!("Player '{}' (id={}) disconnected", player_name, player_id);
        Ok(())
    }
}
