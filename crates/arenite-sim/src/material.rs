use arenite_core::Color;
use arenite_core::registry::Registry;
use serde::{Deserialize, Serialize};
use crate::physics_type::PhysicsType;

/// Static definition of a material type stored in the global registry.
#[derive(Debug, Clone)]
pub struct Material {
    pub display_name: String,
    /// Default physics behaviour.
    pub default_physics: PhysicsType,
    /// Base colour (individual instances may vary).
    pub base_color: Color,
    /// Light emission [r, g, b] in [0, 1].
    pub emission: [f32; 3],
    /// Density: heavier materials sink below lighter ones. Air = 0.
    pub density: f32,
    /// Viscosity for liquids: higher = slower spread.
    pub viscosity: f32,
    /// Spread speed (cells per tick) for fluids and gases.
    pub spread_rate: u8,
    /// Colour jitter amount for variety.
    pub color_jitter: u8,
}

impl Material {
    pub fn builder(display_name: impl Into<String>) -> MaterialBuilder {
        MaterialBuilder {
            display_name:    display_name.into(),
            default_physics: PhysicsType::Air,
            base_color:      Color::TRANSPARENT,
            emission:        [0.0; 3],
            density:         0.0,
            viscosity:       0.0,
            spread_rate:     0,
            color_jitter:    10,
        }
    }
}

pub struct MaterialBuilder {
    display_name:    String,
    default_physics: PhysicsType,
    base_color:      Color,
    emission:        [f32; 3],
    density:         f32,
    viscosity:       f32,
    spread_rate:     u8,
    color_jitter:    u8,
}

impl MaterialBuilder {
    pub fn physics(mut self, p: PhysicsType) -> Self { self.default_physics = p; self }
    pub fn color(mut self, c: Color) -> Self { self.base_color = c; self }
    pub fn emission(mut self, e: [f32; 3]) -> Self { self.emission = e; self }
    pub fn density(mut self, d: f32) -> Self { self.density = d; self }
    pub fn viscosity(mut self, v: f32) -> Self { self.viscosity = v; self }
    pub fn spread_rate(mut self, s: u8) -> Self { self.spread_rate = s; self }
    pub fn jitter(mut self, j: u8) -> Self { self.color_jitter = j; self }

    pub fn build(self) -> Material {
        Material {
            display_name:    self.display_name,
            default_physics: self.default_physics,
            base_color:      self.base_color,
            emission:        self.emission,
            density:         self.density,
            viscosity:       self.viscosity,
            spread_rate:     self.spread_rate,
            color_jitter:    self.color_jitter,
        }
    }
}

/// A concrete pixel instance: combines material ID with per-pixel state.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct MaterialInstance {
    /// Index into the global material registry.
    pub id: u16,
    pub physics: PhysicsType,
    pub color:   Color,
    /// Per-pixel light contribution [r, g, b] in [0, 1].
    pub light:   [f32; 3],
    /// Generic simulation data: temperature, lifetime, etc.
    pub data:    u16,
}

impl MaterialInstance {
    pub const AIR_ID: u16 = 0;

    #[inline]
    pub fn air() -> Self {
        Self {
            id:      Self::AIR_ID,
            physics: PhysicsType::Air,
            color:   Color::TRANSPARENT,
            light:   [0.0; 3],
            data:    0,
        }
    }

    #[inline]
    pub fn is_air(self) -> bool {
        self.physics == PhysicsType::Air
    }

    /// True if this pixel needs to be simulated this tick.
    #[inline]
    pub fn is_dynamic(self) -> bool {
        self.physics.is_dynamic()
    }
}

impl Default for MaterialInstance {
    fn default() -> Self { Self::air() }
}

pub type MaterialRegistry = Registry<Material>;

/// Build the default set of materials for the base game.
pub fn default_material_registry() -> MaterialRegistry {
    let mut r = MaterialRegistry::new();

    r.register("air",   Material::builder("Air")
        .physics(PhysicsType::Air)
        .color(Color::TRANSPARENT)
        .density(0.0)
        .build());

    r.register("dirt",  Material::builder("Dirt")
        .physics(PhysicsType::Solid)
        .color(Color::DIRT)
        .density(1600.0)
        .jitter(15)
        .build());

    r.register("stone", Material::builder("Stone")
        .physics(PhysicsType::Solid)
        .color(Color::STONE)
        .density(2500.0)
        .jitter(8)
        .build());

    r.register("sand",  Material::builder("Sand")
        .physics(PhysicsType::Sand)
        .color(Color::SAND)
        .density(1500.0)
        .jitter(20)
        .build());

    r.register("gravel", Material::builder("Gravel")
        .physics(PhysicsType::Sand)
        .color(Color::rgb(120, 110, 100))
        .density(1700.0)
        .jitter(12)
        .build());

    r.register("water", Material::builder("Water")
        .physics(PhysicsType::Liquid)
        .color(Color::WATER)
        .density(1000.0)
        .spread_rate(6)
        .viscosity(0.1)
        .jitter(5)
        .build());

    r.register("lava",  Material::builder("Lava")
        .physics(PhysicsType::Liquid)
        .color(Color::LAVA)
        .density(2700.0)
        .spread_rate(2)
        .viscosity(0.9)
        .emission([1.0, 0.4, 0.0])
        .jitter(30)
        .build());

    r.register("steam", Material::builder("Steam")
        .physics(PhysicsType::Gas)
        .color(Color::new(200, 200, 220, 80))
        .density(-50.0)
        .spread_rate(3)
        .jitter(20)
        .build());

    r.register("smoke", Material::builder("Smoke")
        .physics(PhysicsType::Gas)
        .color(Color::new(80, 80, 80, 120))
        .density(-20.0)
        .spread_rate(2)
        .jitter(25)
        .build());

    r.register("fire",  Material::builder("Fire")
        .physics(PhysicsType::Fire)
        .color(Color::new(255, 140, 0, 200))
        .density(-100.0)
        .emission([1.0, 0.6, 0.1])
        .jitter(60)
        .build());

    r.register("grass",   Material::builder("Grass")
        .physics(PhysicsType::Solid)
        .color(Color::GRASS)
        .density(1400.0)
        .jitter(12)
        .build());

    r.register("snow",    Material::builder("Snow")
        .physics(PhysicsType::Sand)
        .color(Color::SNOW)
        .density(300.0)
        .jitter(8)
        .build());

    r.register("wood",    Material::builder("Wood")
        .physics(PhysicsType::Solid)
        .color(Color::WOOD)
        .density(600.0)
        .jitter(10)
        .build());

    r.register("gold_ore", Material::builder("Gold Ore")
        .physics(PhysicsType::Solid)
        .color(Color::GOLD)
        .density(3000.0)
        .emission([0.05, 0.04, 0.0])
        .jitter(5)
        .build());

    r.register("iron_ore", Material::builder("Iron Ore")
        .physics(PhysicsType::Solid)
        .color(Color::IRON_ORE)
        .density(2900.0)
        .jitter(8)
        .build());

    r
}
