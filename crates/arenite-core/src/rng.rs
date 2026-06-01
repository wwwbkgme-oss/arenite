/// Lightweight deterministic RNG helpers wrapping `fastrand`.
///
/// # Usage
/// ```ignore
/// let mut rng = AreniteRng::from_seed(42);
/// let v: f32 = rng.f32();
/// ```
pub struct AreniteRng {
    inner: fastrand::Rng,
}

impl AreniteRng {
    pub fn new() -> Self {
        Self { inner: fastrand::Rng::new() }
    }

    pub fn from_seed(seed: u64) -> Self {
        Self { inner: fastrand::Rng::with_seed(seed) }
    }

    /// Seed from two i32 coordinates (useful for per-chunk / per-tile determinism).
    pub fn from_coords(x: i32, y: i32, world_seed: u64) -> Self {
        // FNV-1a inspired mix
        let mut h: u64 = world_seed;
        h ^= (x as u64).wrapping_mul(0x9e3779b97f4a7c15);
        h = h.wrapping_mul(0x6c62272e07bb0142);
        h ^= (y as u64).wrapping_mul(0x517cc1b727220a95);
        h = h.wrapping_mul(0xbf58476d1ce4e5b9);
        h ^= h >> 31;
        Self::from_seed(h)
    }

    #[inline] pub fn bool(&mut self) -> bool { self.inner.bool() }
    #[inline] pub fn u8(&mut self)  -> u8  { self.inner.u8(..) }
    #[inline] pub fn u32(&mut self) -> u32 { self.inner.u32(..) }
    #[inline] pub fn u64(&mut self) -> u64 { self.inner.u64(..) }
    #[inline] pub fn i32_range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.inner.u32(..) % (hi - lo).max(1) as u32) as i32
    }
    #[inline] pub fn f32(&mut self) -> f32 { self.inner.f32() }
    #[inline] pub fn f64(&mut self) -> f64 { self.inner.f64() }

    /// Choose a random element from a non-empty slice.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.inner.usize(..items.len())]
    }

    /// Shuffle a slice in place.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.inner.usize(..=i);
            slice.swap(i, j);
        }
    }
}

impl Default for AreniteRng {
    fn default() -> Self { Self::new() }
}
