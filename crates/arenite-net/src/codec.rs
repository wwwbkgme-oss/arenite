use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::message::{NetMessage, MAX_MESSAGE_BYTES};

/// Write a length-prefixed bincode message to a TCP stream.
pub async fn write_message(
    stream: &mut TcpStream,
    msg:    &NetMessage,
) -> anyhow::Result<()> {
    let bytes = bincode::serialize(msg)?;
    let len   = bytes.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}

/// Read one length-prefixed bincode message from a TCP stream.
pub async fn read_message(stream: &mut TcpStream) -> anyhow::Result<NetMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_BYTES {
        anyhow::bail!("Incoming message too large: {} bytes", len);
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(bincode::deserialize(&buf)?)
}
