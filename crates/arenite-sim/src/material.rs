use crate::physics_type::PhysicsType;
use arenite_core::registry::Registry;
use arenite_core::Color;
use serde::{Deserialize, Serialize};

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
    /// Whether fire can spread to this material (T-035).
    /// True for wood, grass, oil and similar combustibles.
    pub flammable: bool,
    /// Hardness: dig resistance (0 = instant, 255 = indestructible-ish).
    /// Influences how quickly tools can remove this tile.
    pub hardness: u8,
}

impl Material {
    pub fn builder(display_name: impl Into<String>) -> MaterialBuilder {
        MaterialBuilder {
            display_name: display_name.into(),
            default_physics: PhysicsType::Air,
            base_color: Color::TRANSPARENT,
            emission: [0.0; 3],
            density: 0.0,
            viscosity: 0.0,
            spread_rate: 0,
            color_jitter: 10,
            flammable: false,
            hardness: 20,
        }
    }
}

pub struct MaterialBuilder {
    display_name: String,
    default_physics: PhysicsType,
    base_color: Color,
    emission: [f32; 3],
    density: f32,
    viscosity: f32,
    spread_rate: u8,
    color_jitter: u8,
    flammable: bool,
    hardness: u8,
}

impl MaterialBuilder {
    pub fn physics(mut self, p: PhysicsType) -> Self {
        self.default_physics = p;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.base_color = c;
        self
    }
    pub fn emission(mut self, e: [f32; 3]) -> Self {
        self.emission = e;
        self
    }
    pub fn density(mut self, d: f32) -> Self {
        self.density = d;
        self
    }
    pub fn viscosity(mut self, v: f32) -> Self {
        self.viscosity = v;
        self
    }
    pub fn spread_rate(mut self, s: u8) -> Self {
        self.spread_rate = s;
        self
    }
    pub fn jitter(mut self, j: u8) -> Self {
        self.color_jitter = j;
        self
    }
    /// Mark this material as combustible — fire will spread to it (T-035).
    pub fn flammable(mut self) -> Self {
        self.flammable = true;
        self
    }
    /// Set dig hardness (0 = instant, 255 ≈ bedrock).
    pub fn hardness(mut self, h: u8) -> Self {
        self.hardness = h;
        self
    }

    pub fn build(self) -> Material {
        Material {
            display_name: self.display_name,
            default_physics: self.default_physics,
            base_color: self.base_color,
            emission: self.emission,
            density: self.density,
            viscosity: self.viscosity,
            spread_rate: self.spread_rate,
            color_jitter: self.color_jitter,
            flammable: self.flammable,
            hardness: self.hardness,
        }
    }
}

/// A concrete pixel instance: combines material ID with per-pixel state.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct MaterialInstance {
    /// Index into the global material registry.
    pub id: u16,
    pub physics: PhysicsType,
    pub color: Color,
    /// Per-pixel light contribution [r, g, b] in [0, 1].
    pub light: [f32; 3],
    /// Generic simulation data: temperature, lifetime, etc.
    pub data: u16,
}

/// Bit flags stored in the upper byte of `MaterialInstance::data`.
/// The lower byte is available for material-specific simulation state
/// (lifetime, temperature, etc.).
pub mod flags {
    /// Bit 15: this pixel can catch fire (T-035).
    /// Set for wood, grass, oil, and any other combustible material.
    pub const FLAMMABLE: u16 = 0x8000;
    /// Bit 14: this pixel is currently on fire (used by acid corrosion future).
    pub const BURNING: u16 = 0x4000;
}

impl MaterialInstance {
    pub const AIR_ID: u16 = 0;

    #[inline]
    pub fn air() -> Self {
        Self {
            id: Self::AIR_ID,
            physics: PhysicsType::Air,
            color: Color::TRANSPARENT,
            light: [0.0; 3],
            data: 0,
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

    /// True if fire can spread to this pixel (T-035).
    /// The FLAMMABLE bit is set at world-gen / construction time for
    /// combustible materials (wood, grass, oil).
    #[inline]
    pub fn is_flammable(self) -> bool {
        self.data & flags::FLAMMABLE != 0
    }

    /// Return a copy of `self` with the FLAMMABLE bit set.
    #[inline]
    pub fn with_flammable(mut self) -> Self {
        self.data |= flags::FLAMMABLE;
        self
    }

    /// Return simulation lifetime stored in the lower byte of `data`.
    #[inline]
    pub fn lifetime(self) -> u8 {
        self.data as u8
    }

    /// Return a copy with the lower-byte lifetime set to `v`.
    #[inline]
    pub fn with_lifetime(mut self, v: u8) -> Self {
        self.data = (self.data & 0xFF00) | v as u16;
        self
    }
}

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}

pub type MaterialRegistry = Registry<Material>;

impl Material {
    /// Create a `MaterialInstance` from this material definition.
    ///
    /// `id` is the numeric registry ID returned by `Registry::register`.
    /// Sets the `FLAMMABLE` flag from `self.flammable`.
    pub fn to_instance(&self, id: u16) -> MaterialInstance {
        MaterialInstance {
            id,
            physics: self.default_physics,
            color: self.base_color,
            light: self.emission,
            data: if self.flammable { flags::FLAMMABLE } else { 0 },
        }
    }
}

/// Look up a material by string key and return a ready `MaterialInstance`.
/// Returns `None` if the key is not registered.
pub fn make_instance(registry: &MaterialRegistry, key: &str) -> Option<MaterialInstance> {
    let sid = arenite_core::id::StringId::from(key);
    let id = registry.id_of(&sid)?;
    let mat = registry.get_by_id(id)?;
    Some(mat.to_instance(id.raw() as u16))
}

/// Build the default set of materials for the base game.
pub fn default_material_registry() -> MaterialRegistry {
    let mut r = MaterialRegistry::new();

    r.register(
        "air",
        Material::builder("Air")
            .physics(PhysicsType::Air)
            .color(Color::TRANSPARENT)
            .density(0.0)
            .build(),
    );

    r.register(
        "dirt",
        Material::builder("Dirt")
            .physics(PhysicsType::Solid)
            .color(Color::DIRT)
            .density(1600.0)
            .jitter(15)
            .build(),
    );

    r.register(
        "stone",
        Material::builder("Stone")
            .physics(PhysicsType::Solid)
            .color(Color::STONE)
            .density(2500.0)
            .jitter(8)
            .build(),
    );

    r.register(
        "sand",
        Material::builder("Sand")
            .physics(PhysicsType::Sand)
            .color(Color::SAND)
            .density(1500.0)
            .jitter(20)
            .build(),
    );

    r.register(
        "gravel",
        Material::builder("Gravel")
            .physics(PhysicsType::Sand)
            .color(Color::rgb(120, 110, 100))
            .density(1700.0)
            .jitter(12)
            .build(),
    );

    r.register(
        "water",
        Material::builder("Water")
            .physics(PhysicsType::Liquid)
            .color(Color::WATER)
            .density(1000.0)
            .spread_rate(6)
            .viscosity(0.1)
            .jitter(5)
            .build(),
    );

    r.register(
        "lava",
        Material::builder("Lava")
            .physics(PhysicsType::Liquid)
            .color(Color::LAVA)
            .density(2700.0)
            .spread_rate(2)
            .viscosity(0.9)
            .emission([1.0, 0.4, 0.0])
            .jitter(30)
            .build(),
    );

    r.register(
        "steam",
        Material::builder("Steam")
            .physics(PhysicsType::Gas)
            .color(Color::new(200, 200, 220, 80))
            .density(-50.0)
            .spread_rate(3)
            .jitter(20)
            .build(),
    );

    r.register(
        "smoke",
        Material::builder("Smoke")
            .physics(PhysicsType::Gas)
            .color(Color::new(80, 80, 80, 120))
            .density(-20.0)
            .spread_rate(2)
            .jitter(25)
            .build(),
    );

    r.register(
        "fire",
        Material::builder("Fire")
            .physics(PhysicsType::Fire)
            .color(Color::new(255, 140, 0, 200))
            .density(-100.0)
            .emission([1.0, 0.6, 0.1])
            .jitter(60)
            .build(),
    );

    r.register(
        "grass",
        Material::builder("Grass")
            .physics(PhysicsType::Solid)
            .color(Color::GRASS)
            .density(1400.0)
            .jitter(12)
            .flammable() // Grass burns — T-035
            .hardness(10)
            .build(),
    );

    r.register(
        "snow",
        Material::builder("Snow")
            .physics(PhysicsType::Sand)
            .color(Color::SNOW)
            .density(300.0)
            .jitter(8)
            .hardness(5)
            .build(),
    );

    r.register(
        "wood",
        Material::builder("Wood")
            .physics(PhysicsType::Solid)
            .color(Color::WOOD)
            .density(600.0)
            .jitter(10)
            .flammable() // Wood burns — T-035
            .hardness(30)
            .build(),
    );

    r.register(
        "gold_ore",
        Material::builder("Gold Ore")
            .physics(PhysicsType::Solid)
            .color(Color::GOLD)
            .density(3000.0)
            .emission([0.05, 0.04, 0.0])
            .jitter(5)
            .hardness(60)
            .build(),
    );

    r.register(
        "iron_ore",
        Material::builder("Iron Ore")
            .physics(PhysicsType::Solid)
            .color(Color::IRON_ORE)
            .density(2900.0)
            .jitter(8)
            .hardness(55)
            .build(),
    );

    // ── New materials — Terraria / Starbound / re-flora inspired ─────────

    // Clay: found near water bodies; yields clay items when mined (SDV, Terraria).
    r.register(
        "clay",
        Material::builder("Clay")
            .physics(PhysicsType::Solid)
            .color(Color::CLAY)
            .density(1700.0)
            .jitter(10)
            .hardness(18)
            .build(),
    );

    // Mud: jungle floor material; mushrooms grow on it (Terraria mud).
    r.register(
        "mud",
        Material::builder("Mud")
            .physics(PhysicsType::Solid)
            .color(Color::MUD)
            .density(1550.0)
            .jitter(20)
            .hardness(12)
            .build(),
    );

    // Obsidian: extremely hard volcanic glass; formed when lava meets water (T-034).
    r.register(
        "obsidian",
        Material::builder("Obsidian")
            .physics(PhysicsType::Solid)
            .color(Color::OBSIDIAN)
            .density(3200.0)
            .jitter(4)
            .hardness(180)
            .build(),
    );

    // Oil: dark flammable liquid; floats on water (less dense); Starbound fuel.
    r.register(
        "oil",
        Material::builder("Oil")
            .physics(PhysicsType::Liquid)
            .color(Color::OIL)
            .density(850.0) // lighter than water → floats
            .spread_rate(4)
            .viscosity(0.5)
            .jitter(8)
            .flammable() // Oil burns — T-035
            .hardness(0)
            .build(),
    );

    // Acid: corrosive green liquid; high spread; Starbound alien biomes.
    r.register(
        "acid",
        Material::builder("Acid")
            .physics(PhysicsType::Liquid)
            .color(Color::ACID)
            .density(1100.0)
            .spread_rate(7)
            .viscosity(0.15)
            .emission([0.0, 0.2, 0.0])
            .jitter(15)
            .hardness(0)
            .build(),
    );

    // Ice: granular powder physics; slides downhill; cold biome surface.
    // Behaves like Sand but much lighter (Terraria ice block).
    r.register(
        "ice",
        Material::builder("Ice")
            .physics(PhysicsType::Sand)
            .color(Color::ICE)
            .density(920.0) // slightly lighter than water → floats when melted
            .jitter(5)
            .hardness(15)
            .build(),
    );

    // Crystal: emissive translucent solid; Starbound crystal cave biome.
    r.register(
        "crystal",
        Material::builder("Crystal")
            .physics(PhysicsType::Solid)
            .color(Color::CRYSTAL)
            .density(2200.0)
            .emission([0.0, 0.6, 0.9])
            .jitter(12)
            .hardness(70)
            .build(),
    );

    // Mushroom block: spongy purple solid; Terraria glowing mushroom biome.
    r.register(
        "mushroom_block",
        Material::builder("Mushroom Block")
            .physics(PhysicsType::Solid)
            .color(Color::MUSHROOM)
            .density(500.0)
            .emission([0.05, 0.0, 0.1])
            .jitter(15)
            .hardness(12)
            .build(),
    );

    // Copper ore: reddish-orange metallic ore (Terraria / Starbound tier-1).
    r.register(
        "copper_ore",
        Material::builder("Copper Ore")
            .physics(PhysicsType::Solid)
            .color(Color::COPPER_ORE)
            .density(2800.0)
            .jitter(10)
            .hardness(45)
            .build(),
    );

    // Titanium ore: silver-white high-tier ore (Starbound tier-3).
    r.register(
        "titanium_ore",
        Material::builder("Titanium Ore")
            .physics(PhysicsType::Solid)
            .color(Color::TITANIUM)
            .density(4000.0)
            .emission([0.03, 0.03, 0.05])
            .jitter(6)
            .hardness(120)
            .build(),
    );

    // Diamond: rare deep gem; Terraria / Starbound precious gem.
    r.register(
        "diamond",
        Material::builder("Diamond")
            .physics(PhysicsType::Solid)
            .color(Color::DIAMOND)
            .density(3500.0)
            .emission([0.1, 0.15, 0.2])
            .jitter(3)
            .hardness(150)
            .build(),
    );

    r
}
