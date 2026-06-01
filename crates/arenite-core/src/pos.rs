use glam::IVec2;

/// The number of pixels along each axis of a single chunk.
pub const CHUNK_SIZE: i32 = 64;
pub const CHUNK_AREA: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// A position measured in individual pixels (world-space).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Which chunk contains this tile.
    #[inline]
    pub fn to_chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(CHUNK_SIZE),
            y: self.y.div_euclid(CHUNK_SIZE),
        }
    }

    /// Local position within the owning chunk [0, CHUNK_SIZE).
    #[inline]
    pub fn local(self) -> (i32, i32) {
        (self.x.rem_euclid(CHUNK_SIZE), self.y.rem_euclid(CHUNK_SIZE))
    }

    /// Convert to flat index within a chunk (bounds are NOT checked).
    #[inline]
    pub fn local_index(self) -> usize {
        let (lx, ly) = self.local();
        (ly * CHUNK_SIZE + lx) as usize
    }
}

impl From<IVec2> for TilePos {
    fn from(v: IVec2) -> Self {
        Self::new(v.x, v.y)
    }
}
impl From<TilePos> for IVec2 {
    fn from(t: TilePos) -> Self {
        IVec2::new(t.x, t.y)
    }
}

/// A position measured in chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
}

impl ChunkPos {
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Top-left tile position of this chunk.
    #[inline]
    pub fn origin_tile(self) -> TilePos {
        TilePos {
            x: self.x * CHUNK_SIZE,
            y: self.y * CHUNK_SIZE,
        }
    }

    #[inline]
    pub fn neighbours(self) -> [ChunkPos; 8] {
        [
            ChunkPos::new(self.x - 1, self.y - 1),
            ChunkPos::new(self.x, self.y - 1),
            ChunkPos::new(self.x + 1, self.y - 1),
            ChunkPos::new(self.x - 1, self.y),
            ChunkPos::new(self.x + 1, self.y),
            ChunkPos::new(self.x - 1, self.y + 1),
            ChunkPos::new(self.x, self.y + 1),
            ChunkPos::new(self.x + 1, self.y + 1),
        ]
    }

    pub fn manhattan_distance(self, other: ChunkPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl From<IVec2> for ChunkPos {
    fn from(v: IVec2) -> Self {
        Self::new(v.x, v.y)
    }
}
impl From<ChunkPos> for IVec2 {
    fn from(c: ChunkPos) -> Self {
        IVec2::new(c.x, c.y)
    }
}

/// A continuous floating-point position in the world (pixels as f32).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorldPos {
    pub x: f32,
    pub y: f32,
}

impl WorldPos {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    #[inline]
    pub fn tile(self) -> TilePos {
        TilePos::new(self.x.floor() as i32, self.y.floor() as i32)
    }
    #[inline]
    pub fn chunk(self) -> ChunkPos {
        self.tile().to_chunk()
    }
    #[inline]
    pub fn to_vec2(self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }
}

impl From<glam::Vec2> for WorldPos {
    fn from(v: glam::Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}
