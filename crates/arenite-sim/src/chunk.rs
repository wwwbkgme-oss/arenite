use arenite_core::pos::{CHUNK_SIZE, CHUNK_AREA};
use crate::material::MaterialInstance;

/// Tracks the smallest rectangle enclosing all modified pixels this tick.
/// Used to avoid redrawing unchanged GPU textures.
#[derive(Clone, Copy, Debug)]
pub struct DirtyRect {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub dirty: bool,
}

impl DirtyRect {
    pub fn clean() -> Self {
        Self {
            min_x: CHUNK_SIZE,
            min_y: CHUNK_SIZE,
            max_x: 0,
            max_y: 0,
            dirty: false,
        }
    }

    #[inline]
    pub fn mark(&mut self, x: i32, y: i32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x + 1);
        self.max_y = self.max_y.max(y + 1);
        self.dirty = true;
    }

    pub fn merge(&mut self, other: DirtyRect) {
        if !other.dirty { return; }
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
        self.dirty = true;
    }
}

/// All simulation data for one `CHUNK_SIZE × CHUNK_SIZE` tile region.
#[derive(Clone)]
pub struct ChunkData {
    /// Flat array of pixel instances, row-major: index = y * CHUNK_SIZE + x
    pub pixels: Vec<MaterialInstance>,
    /// Mirrors pixels; updated by the lighting pass.
    pub light:  Vec<[f32; 3]>,
    /// Dirty rect since last GPU upload.
    pub dirty:  DirtyRect,
}

impl ChunkData {
    pub fn new_empty() -> Self {
        Self {
            pixels: vec![MaterialInstance::air(); CHUNK_AREA],
            light:  vec![[0.0_f32; 3]; CHUNK_AREA],
            dirty:  DirtyRect::clean(),
        }
    }

    /// Return pixel index for chunk-local (x, y), unchecked.
    #[inline]
    pub fn idx(x: i32, y: i32) -> usize {
        debug_assert!((0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y));
        (y * CHUNK_SIZE + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> MaterialInstance {
        self.pixels[Self::idx(x, y)]
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        let idx = Self::idx(x, y);
        self.pixels[idx] = mat;
        self.dirty.mark(x, y);
    }

    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, mat: MaterialInstance) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y) {
                    self.set(x, y, mat);
                }
            }
        }
    }

    /// Count non-air pixels.
    pub fn voxel_count(&self) -> usize {
        self.pixels.iter().filter(|p| !p.is_air()).count()
    }
}

/// A simulation chunk: wraps `ChunkData` with position metadata.
pub struct Chunk {
    pub pos:  arenite_core::pos::ChunkPos,
    pub data: ChunkData,
    /// Has this chunk been loaded/generated?
    pub loaded: bool,
    /// Should this chunk be simulated?
    pub active: bool,
    /// Frame number of last simulation tick (for determinism).
    pub last_tick: u64,
}

impl Chunk {
    pub fn new(pos: arenite_core::pos::ChunkPos) -> Self {
        Self {
            pos,
            data:      ChunkData::new_empty(),
            loaded:    false,
            active:    true,
            last_tick: 0,
        }
    }
}
