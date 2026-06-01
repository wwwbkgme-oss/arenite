/// RGBA colour stored as four `u8` values.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK:       Self = Self { r: 0,   g: 0,   b: 0,   a: 255 };
    pub const WHITE:       Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const RED:         Self = Self { r: 255, g: 0,   b: 0,   a: 255 };
    pub const GREEN:       Self = Self { r: 0,   g: 255, b: 0,   a: 255 };
    pub const BLUE:        Self = Self { r: 0,   g: 0,   b: 255, a: 255 };
    pub const DIRT:        Self = Self { r: 139, g: 90,  b: 43,  a: 255 };
    pub const STONE:       Self = Self { r: 128, g: 128, b: 128, a: 255 };
    pub const SAND:        Self = Self { r: 219, g: 191, b: 116, a: 255 };
    pub const WATER:       Self = Self { r: 40,  g: 100, b: 200, a: 200 };
    pub const LAVA:        Self = Self { r: 220, g: 80,  b: 20,  a: 255 };
    pub const GRASS:       Self = Self { r: 76,  g: 153, b: 0,   a: 255 };
    pub const SNOW:        Self = Self { r: 230, g: 240, b: 255, a: 255 };
    pub const WOOD:        Self = Self { r: 133, g: 94,  b: 66,  a: 255 };
    pub const GOLD:        Self = Self { r: 255, g: 200, b: 50,  a: 255 };
    pub const IRON_ORE:    Self = Self { r: 153, g: 102, b: 0,   a: 255 };

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Linearly interpolate between two colours.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let lerp_u8 = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
        Self {
            r: lerp_u8(self.r, other.r),
            g: lerp_u8(self.g, other.g),
            b: lerp_u8(self.b, other.b),
            a: lerp_u8(self.a, other.a),
        }
    }

    /// Multiply RGB by a factor (for lighting), clamped to [0, 255].
    pub fn multiply(self, factor: f32) -> Self {
        let mul = |v: u8| (v as f32 * factor).min(255.0) as u8;
        Self { r: mul(self.r), g: mul(self.g), b: mul(self.b), a: self.a }
    }

    /// Convert to `[f32; 4]` normalised RGBA.
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// Add small random jitter for visual variety (keeps pixel art feeling alive).
    pub fn jitter(self, rng: &mut impl rand::Rng, amount: u8) -> Self {
        let jit = |v: u8| {
            let delta = rng.gen_range(0..=amount as i16) - (amount / 2) as i16;
            (v as i16 + delta).clamp(0, 255) as u8
        };
        Self { r: jit(self.r), g: jit(self.g), b: jit(self.b), a: self.a }
    }
}

impl From<Color> for u32 {
    fn from(c: Color) -> u32 {
        ((c.a as u32) << 24)
            | ((c.b as u32) << 16)
            | ((c.g as u32) << 8)
            | c.r as u32
    }
}

impl From<u32> for Color {
    fn from(v: u32) -> Self {
        Self {
            r: (v & 0xFF) as u8,
            g: ((v >> 8) & 0xFF) as u8,
            b: ((v >> 16) & 0xFF) as u8,
            a: ((v >> 24) & 0xFF) as u8,
        }
    }
}
