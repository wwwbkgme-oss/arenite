use thiserror::Error;

/// Top-level engine error type.
#[derive(Debug, Error)]
pub enum AreniteError {
    #[error("chunk not found: {0:?}")]
    ChunkNotFound(crate::pos::ChunkPos),

    #[error("world I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialisation: {0}")]
    Serialise(String),

    #[error("invalid material id {0}")]
    InvalidMaterial(u16),

    #[error("network: {0}")]
    Network(String),

    #[error("{0}")]
    Other(String),
}

impl From<Box<bincode::ErrorKind>> for AreniteError {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        AreniteError::Serialise(e.to_string())
    }
}

pub type AreniteResult<T> = Result<T, AreniteError>;
