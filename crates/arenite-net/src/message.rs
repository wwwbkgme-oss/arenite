use arenite_core::pos::{ChunkPos, TilePos, WorldPos};
use arenite_sim::material::MaterialInstance;
use serde::{Deserialize, Serialize};

/// Player input state sent from client to server each tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    /// World-space cursor position (for placing/removing pixels).
    pub cursor: WorldPos,
    /// True if the primary action (place material) is held.
    pub placing: bool,
    /// True if the secondary action (remove material) is held.
    pub removing: bool,
    /// Selected material index.
    pub selected_mat: u16,
    /// Brush radius in pixels.
    pub brush_radius: i32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            left: false,
            right: false,
            up: false,
            down: false,
            jump: false,
            cursor: WorldPos::new(0.0, 0.0),
            placing: false,
            removing: false,
            selected_mat: 0,
            brush_radius: 3,
        }
    }
}

/// All messages exchanged between client and server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // ── Handshake ──────────────────────────────────────────────────────────
    /// Client → Server: request to join the game.
    Hello {
        protocol_version: u16,
        player_name: String,
    },
    /// Server → Client: accept connection, assign player ID.
    Welcome {
        player_id: u32,
        world_seed: u64,
        world_width: i32,
        world_height: i32,
        spawn_x: f32,
        spawn_y: f32,
    },
    /// Server → Client: connection rejected.
    Reject {
        reason: String,
    },

    // ── World data ─────────────────────────────────────────────────────────
    /// Server → Client: raw chunk pixel data.
    ChunkData {
        pos: ChunkPos,
        /// RGBA8 flat buffer: CHUNK_AREA × 4 bytes.
        pixels: Vec<u8>,
    },
    /// Server → Client: a single pixel has changed.
    PixelUpdate {
        pos: TilePos,
        mat: MaterialInstance,
    },

    // ── Player state ───────────────────────────────────────────────────────
    /// Client → Server: input this tick.
    Input(InputState),
    /// Server → Client: updated player position.
    PlayerPos {
        player_id: u32,
        x: f32,
        y: f32,
        vel_x: f32,
        vel_y: f32,
    },
    /// Server → Client: another player connected.
    PlayerJoined {
        player_id: u32,
        name: String,
    },
    /// Server → Client: player left.
    PlayerLeft {
        player_id: u32,
    },

    // ── Ping ───────────────────────────────────────────────────────────────
    Ping {
        sequence: u32,
    },
    Pong {
        sequence: u32,
    },

    // ── Graceful disconnect ────────────────────────────────────────────────
    Disconnect,
}

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024; // 4 MiB
