// arenite-net — client/server networking
//
// Provides a lightweight TCP-framed message protocol for multiplayer.
// Inspired by FallingSandEngine's common/networking and rustaria's
// client/server split.
//
// Protocol
// ========
// Each message is preceded by a u32 little-endian byte length header,
// followed by bincode-serialised `NetMessage` bytes.  Both client and
// server share the same message enum and framing layer.

pub mod codec;
pub mod message;
pub mod server;
pub mod client;

pub use message::NetMessage;
pub use server::GameServer;
pub use client::GameClient;
