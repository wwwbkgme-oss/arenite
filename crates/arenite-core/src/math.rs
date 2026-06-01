/// Shared math helpers on top of `glam`.
pub use glam::{IVec2, UVec2, Vec2, Vec3, Vec4};

/// Integer rectangle (pixel coords).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IRect {
    pub min: IVec2,
    pub max: IVec2,
}

impl IRect {
    pub fn new(min: IVec2, max: IVec2) -> Self {
        Self { min, max }
    }
    pub fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            min: IVec2::new(x, y),
            max: IVec2::new(x + w, y + h),
        }
    }
    pub fn width(self) -> i32 {
        self.max.x - self.min.x
    }
    pub fn height(self) -> i32 {
        self.max.y - self.min.y
    }
    pub fn contains(self, p: IVec2) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }
    pub fn area(self) -> i32 {
        self.width() * self.height()
    }
}

/// Float rectangle (world coords).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            max: Vec2::new(x + w, y + h),
        }
    }
    pub fn centre(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }
}

/// Signed-integer direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction4 {
    Up,
    Down,
    Left,
    Right,
}

impl Direction4 {
    pub fn offset(self) -> IVec2 {
        match self {
            Self::Up => IVec2::new(0, -1),
            Self::Down => IVec2::new(0, 1),
            Self::Left => IVec2::new(-1, 0),
            Self::Right => IVec2::new(1, 0),
        }
    }
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];
}

/// Clamp an integer between inclusive bounds.
#[inline]
pub fn clamp_i32(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi)
}

/// Linear interpolation for f32.
#[inline]
pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smooth-step (Hermite) remapping.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
