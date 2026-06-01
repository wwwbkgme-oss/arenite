use log::{info, warn};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::codec::{read_message, write_message};
use crate::message::{NetMessage, PROTOCOL_VERSION};

/// Client-side connection to an Arenite game server.
pub struct GameClient {
    pub player_id: u32,
    pub player_name: String,
    pub server_addr: SocketAddr,
    /// Channel for sending messages from game logic → network task.
    pub send_tx: mpsc::UnboundedSender<NetMessage>,
    /// Channel for receiving messages from network task → game logic.
    pub recv_rx: mpsc::UnboundedReceiver<NetMessage>,
}

impl GameClient {
    /// Connect to a server, complete the handshake, and return a ready client.
    pub async fn connect(server_addr: SocketAddr, player_name: String) -> anyhow::Result<Self> {
        let mut stream = TcpStream::connect(server_addr).await?;
        info!("Connected to server at {}", server_addr);

        // Send Hello.
        write_message(
            &mut stream,
            &NetMessage::Hello {
                protocol_version: PROTOCOL_VERSION,
                player_name: player_name.clone(),
            },
        )
        .await?;

        // Wait for Welcome or Reject.
        let player_id = match read_message(&mut stream).await? {
            NetMessage::Welcome {
                player_id,
                world_seed,
                world_width,
                world_height,
                spawn_x,
                spawn_y,
            } => {
                info!(
                    "Welcomed as id={}, world={}×{}, seed={}, spawn=({},{})",
                    player_id, world_width, world_height, world_seed, spawn_x, spawn_y
                );
                player_id
            }
            NetMessage::Reject { reason } => {
                anyhow::bail!("Server rejected connection: {}", reason);
            }
            other => anyhow::bail!("Unexpected handshake message: {:?}", other),
        };

        // Spawn background send/receive tasks.
        let (send_tx, mut send_rx) = mpsc::unbounded_channel::<NetMessage>();
        let (recv_tx, recv_rx) = mpsc::unbounded_channel::<NetMessage>();

        let (mut reader, mut writer) = stream.into_split();

        // Write task: forward queued outbound messages to TCP.
        tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                let bytes = match bincode::serialize(&msg) {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("Serialise error: {}", e);
                        continue;
                    }
                };
                let len = bytes.len() as u32;
                use tokio::io::AsyncWriteExt;
                if writer.write_all(&len.to_le_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(&bytes).await.is_err() {
                    break;
                }
            }
        });

        // Read task: forward inbound messages to the game logic channel.
        tokio::spawn(async move {
            loop {
                let mut len_buf = [0u8; 4];
                use tokio::io::AsyncReadExt;
                if reader.read_exact(&mut len_buf).await.is_err() {
                    break;
                }
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                if reader.read_exact(&mut buf).await.is_err() {
                    break;
                }
                match bincode::deserialize::<NetMessage>(&buf) {
                    Ok(msg) => {
                        let _ = recv_tx.send(msg);
                    }
                    Err(e) => warn!("Deserialise error: {}", e),
                }
            }
        });

        Ok(Self {
            player_id,
            player_name,
            server_addr,
            send_tx,
            recv_rx,
        })
    }

    /// Send a message to the server (non-blocking).
    pub fn send(&self, msg: NetMessage) {
        let _ = self.send_tx.send(msg);
    }

    /// Poll for any pending inbound messages (call every game tick).
    pub fn poll_messages(&mut self) -> Vec<NetMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.recv_rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }

    pub fn disconnect(&self) {
        let _ = self.send_tx.send(NetMessage::Disconnect);
    }
}
