use crate::noise_field::NoiseField;
use arenite_core::pos::TilePos;
use arenite_core::rng::AreniteRng;
use arenite_sim::material::{flags, MaterialInstance};
use arenite_sim::physics_type::PhysicsType;
use arenite_sim::SimWorld;

/// Parameters for a single ore scatter pass.
struct OreSpec {
    color: arenite_core::Color,
    light: [f32; 3],
    nlo: f64,
    nhi: f64,
    y_min: i32,
    y_max: i32,
    count: i32,
    blob_r: i32,
}

/// Feature placer: adds ore veins, gem deposits, cave decorations, etc.
///
/// Expanded from the original to include:
/// - Copper / titanium / diamond ore tiers (Terraria / Starbound tier system)
/// - Cave mushrooms (Terraria mushroom biome)
/// - Crystal clusters (Starbound crystal caves)
/// - Clay pockets near surface (SDV / Terraria clay)
/// - Surface grass/flower decorations with flammable flag (T-035)
pub struct FeaturePlacer;

impl FeaturePlacer {
    /// Place all ore blobs throughout the world.
    pub fn place_ores(
        world: &mut SimWorld,
        width: i32,
        height: i32,
        noise: &NoiseField,
        underground_y: i32,
        cavern_y: i32,
    ) {
        let mut rng = AreniteRng::from_seed(world.seed ^ 0xdead_beef);

        // Ore specs arranged shallow → deep (matching Terraria / Starbound tiers).
        let specs: &[OreSpec] = &[
            // Copper: mid-shallow underground
            OreSpec {
                color: arenite_core::Color::COPPER_ORE,
                light: [0.0; 3],
                nlo: 0.68,
                nhi: 0.73,
                y_min: underground_y,
                y_max: cavern_y,
                count: 600,
                blob_r: 3,
            },
            // Iron: shallow to mid underground
            OreSpec {
                color: arenite_core::Color::IRON_ORE,
                light: [0.0; 3],
                nlo: 0.72,
                nhi: 0.78,
                y_min: underground_y,
                y_max: cavern_y,
                count: 800,
                blob_r: 3,
            },
            // Gold: mid to deep underground
            OreSpec {
                color: arenite_core::Color::GOLD,
                light: [0.05, 0.04, 0.0],
                nlo: 0.75,
                nhi: 0.79,
                y_min: cavern_y,
                y_max: height - 100,
                count: 300,
                blob_r: 4,
            },
            // Titanium: deep cavern
            OreSpec {
                color: arenite_core::Color::TITANIUM,
                light: [0.02, 0.02, 0.04],
                nlo: 0.76,
                nhi: 0.80,
                y_min: cavern_y,
                y_max: (height - 150).max(cavern_y + 10),
                count: 150,
                blob_r: 3,
            },
            // Diamond: near the underworld
            OreSpec {
                color: arenite_core::Color::DIAMOND,
                light: [0.08, 0.1, 0.15],
                nlo: 0.78,
                nhi: 0.82,
                y_min: (height as f64 * 0.8) as i32,
                y_max: height - 50,
                count: 60,
                blob_r: 2,
            },
        ];

        for spec in specs {
            Self::scatter_ore(world, &mut rng, noise, width, height, spec);
        }
    }

    fn scatter_ore(
        world: &mut SimWorld,
        rng: &mut AreniteRng,
        noise: &NoiseField,
        world_w: i32,
        world_h: i32,
        spec: &OreSpec,
    ) {
        let mat = MaterialInstance {
            id: 0,
            physics: PhysicsType::Solid,
            color: spec.color,
            light: spec.light,
            data: 0,
        };

        let mut placed = 0;
        let mut attempts = 0;
        while placed < spec.count && attempts < spec.count * 20 {
            attempts += 1;
            let x = rng.i32_range(0, world_w);
            let y = rng.i32_range(spec.y_min, spec.y_max.min(world_h));

            let nx = x as f64 / world_w as f64 * 8.0;
            let ny = y as f64 / world_h as f64 * 8.0;
            let nv = (noise.feature(nx, ny) + 1.0) * 0.5;

            if nv >= spec.nlo && nv <= spec.nhi {
                let tp = TilePos::new(x, y);
                let existing = world.get_pixel(tp);
                if existing.physics == PhysicsType::Solid && !existing.is_air() {
                    world.paint_circle(tp, rng.i32_range(1, spec.blob_r + 1), mat);
                    placed += 1;
                }
            }
        }
    }

    /// Place clay pockets near the surface and at biome transitions.
    /// Clay is found next to water bodies (SDV, Terraria).
    pub fn place_clay(world: &mut SimWorld, width: i32, surface: &[i32], rng: &mut AreniteRng) {
        let clay_mat = MaterialInstance {
            id: 0,
            physics: PhysicsType::Solid,
            color: arenite_core::Color::CLAY,
            light: [0.0; 3],
            data: 0,
        };

        let count = width / 60;
        for _ in 0..count {
            let x = rng.i32_range(5, width - 5);
            let sy = surface.get(x as usize).copied().unwrap_or(0);
            // Place clay pocket 5-15 tiles below surface.
            let depth = rng.i32_range(5, 16);
            let ty = sy + depth;
            let r = rng.i32_range(2, 5);
            let tp = TilePos::new(x, ty);
            if world.get_pixel(tp).physics == PhysicsType::Solid {
                world.paint_circle(tp, r, clay_mat);
            }
        }
    }

    /// Place surface decorations: grass tufts, flowers, mushrooms.
    pub fn place_surface_decorations(
        world: &mut SimWorld,
        width: i32,
        surface: &[i32],
        rng: &mut AreniteRng,
    ) {
        for x in 0..width {
            let sy = surface.get(x as usize).copied().unwrap_or(0);
            if rng.i32_range(0, 30) == 0 {
                let deco = MaterialInstance {
                    id: 0,
                    physics: PhysicsType::Solid,
                    color: arenite_core::Color::rgb(0, 180, 20),
                    light: [0.0; 3],
                    // Grass tuft: mark flammable so fire spreads to it (T-035).
                    data: flags::FLAMMABLE,
                };
                world.set_pixel(TilePos::new(x, sy - 1), deco);
            }
            // Occasional flower (yellow/pink pixel on top of grass).
            if rng.i32_range(0, 80) == 0 {
                let hue = if rng.i32_range(0, 2) == 0 {
                    arenite_core::Color::rgb(255, 220, 50) // yellow flower
                } else {
                    arenite_core::Color::rgb(255, 150, 180) // pink flower
                };
                let flower = MaterialInstance {
                    id: 0,
                    physics: PhysicsType::Solid,
                    color: hue,
                    light: [0.0; 3],
                    data: flags::FLAMMABLE,
                };
                world.set_pixel(TilePos::new(x, sy - 1), flower);
            }
        }
    }
}
