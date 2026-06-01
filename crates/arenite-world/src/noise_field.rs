use noise::{NoiseFn, OpenSimplex, Perlin, Fbm, MultiFractal};

/// Wraps multiple noise generators needed during world generation.
///
/// Inspired by terra-awg's `Random::initNoise()` which pre-samples seamless
/// 2D noise on a toroidal surface using 4D OpenSimplex, giving a world that
/// wraps horizontally with no visible seam.
pub struct NoiseField {
    pub seed: u64,
    /// Fine-grained terrain detail.
    fbm_fine:   Fbm<OpenSimplex>,
    /// Coarse terrain shape.
    fbm_coarse: Fbm<OpenSimplex>,
    /// Cave carving noise.
    cave:       Fbm<Perlin>,
    /// Biome selector.
    biome:      OpenSimplex,
    /// Ore/feature noise.
    feature:    Perlin,
}

impl NoiseField {
    pub fn new(seed: u64) -> Self {
        let seed32 = (seed ^ (seed >> 32)) as u32;

        let fbm_fine = Fbm::<OpenSimplex>::new(seed32)
            .set_octaves(5)
            .set_frequency(1.0)
            .set_persistence(0.5)
            .set_lacunarity(2.0);

        let fbm_coarse = Fbm::<OpenSimplex>::new(seed32.wrapping_add(1))
            .set_octaves(3)
            .set_frequency(1.0)
            .set_persistence(0.6)
            .set_lacunarity(2.1);

        let cave = Fbm::<Perlin>::new(seed32.wrapping_add(2))
            .set_octaves(4)
            .set_frequency(1.0)
            .set_persistence(0.55)
            .set_lacunarity(2.0);

        Self {
            seed,
            fbm_fine,
            fbm_coarse,
            cave,
            biome:   OpenSimplex::new(seed32.wrapping_add(3)),
            feature: Perlin::new(seed32.wrapping_add(4)),
        }
    }

    /// Sample the fine terrain noise at normalised world coordinates.
    #[inline]
    pub fn terrain_fine(&self, nx: f64, ny: f64) -> f64 {
        self.fbm_fine.get([nx, ny])
    }

    /// Sample the coarse shape noise.
    #[inline]
    pub fn terrain_coarse(&self, nx: f64, ny: f64) -> f64 {
        self.fbm_coarse.get([nx, ny])
    }

    /// Combined terrain noise: coarse shape + fine detail overlay.
    pub fn terrain(&self, nx: f64, ny: f64) -> f64 {
        let c = self.terrain_coarse(nx * 0.4, ny * 0.4);
        let f = self.terrain_fine (nx,        ny       ) * 0.3;
        (c + f).clamp(-1.0, 1.0)
    }

    /// Cave noise at two frequencies — high values indicate open space.
    /// This uses the "domain warped + two-layer" technique from
    /// FallingSandEngine's cave populator.
    pub fn cave(&self, nx: f64, ny: f64) -> f64 {
        // Domain warp: offset sample position by another noise layer.
        let wx = self.fbm_fine.get([nx * 0.7, ny * 0.7]) * 0.3;
        let wy = self.fbm_fine.get([nx * 0.7 + 100.0, ny * 0.7]) * 0.3;
        let v1 = self.cave.get([nx + wx, ny + wy]);
        // Second pass at a different scale for nested cave shapes.
        let v2 = self.cave.get([nx * 2.1 + 50.0, ny * 2.1]) * 0.5;
        v1 * v1 + v2 * v2   // squaring creates sharp-edged tube shapes
    }

    /// Biome selector noise in [0, 1].
    pub fn biome_selector(&self, nx: f64) -> f64 {
        (self.biome.get([nx, 0.0]) + 1.0) * 0.5
    }

    /// Ore / feature placement noise.
    pub fn feature(&self, nx: f64, ny: f64) -> f64 {
        self.feature.get([nx, ny])
    }
}
