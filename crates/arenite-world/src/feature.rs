use arenite_core::pos::TilePos;
use arenite_core::rng::AreniteRng;
use arenite_sim::SimWorld;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;
use crate::noise_field::NoiseField;

/// Parameters for a single ore scatter pass.
struct OreSpec<'a> {
    name:    &'a str,
    nlo:     f64,
    nhi:     f64,
    y_min:   i32,
    y_max:   i32,
    count:   i32,
    blob_r:  i32,
}

/// Feature placer: adds ore veins, gem deposits, gravel patches, etc.
///
/// Inspired by FallingSandEngine's feature/placement_mods pipeline and
/// terra-awg's ore scatter logic.
pub struct FeaturePlacer;

impl FeaturePlacer {
    /// Place ore blobs throughout the world.
    pub fn place_ores(
        world:         &mut SimWorld,
        width:         i32,
        height:        i32,
        noise:         &NoiseField,
        underground_y: i32,
        cavern_y:      i32,
    ) {
        let mut rng = AreniteRng::from_seed(world.seed ^ 0xdead_beef);
        let specs = [
            OreSpec {
                name: "iron_ore", nlo: 0.72, nhi: 0.78,
                y_min: underground_y, y_max: cavern_y,
                count: 800, blob_r: 3,
            },
            OreSpec {
                name: "gold_ore", nlo: 0.75, nhi: 0.79,
                y_min: cavern_y, y_max: height - 100,
                count: 300, blob_r: 4,
            },
        ];
        for spec in &specs {
            Self::scatter_ore(world, &mut rng, noise, width, height, spec);
        }
    }

    fn scatter_ore(
        world:   &mut SimWorld,
        rng:     &mut AreniteRng,
        noise:   &NoiseField,
        world_w: i32,
        world_h: i32,
        spec:    &OreSpec<'_>,
    ) {
        let mat = MaterialInstance {
            id:      0,
            physics: PhysicsType::Solid,
            color:   if spec.name.contains("gold") {
                arenite_core::Color::GOLD
            } else {
                arenite_core::Color::IRON_ORE
            },
            light: [0.0; 3],
            data:  0,
        };

        let mut placed   = 0;
        let mut attempts = 0;
        while placed < spec.count && attempts < spec.count * 20 {
            attempts += 1;
            let x = rng.i32_range(0, world_w);
            let y = rng.i32_range(spec.y_min, spec.y_max.min(world_h));

            let nx = x as f64 / world_w as f64 * 8.0;
            let ny = y as f64 / world_h as f64 * 8.0;
            let nv = (noise.feature(nx, ny) + 1.0) * 0.5;

            if nv >= spec.nlo && nv <= spec.nhi {
                let tp       = TilePos::new(x, y);
                let existing = world.get_pixel(tp);
                if existing.physics == PhysicsType::Solid && !existing.is_air() {
                    world.paint_circle(tp, rng.i32_range(1, spec.blob_r + 1), mat);
                    placed += 1;
                }
            }
        }
    }

    /// Place surface decorations: grass tufts, flowers, mushrooms.
    pub fn place_surface_decorations(
        world:   &mut SimWorld,
        width:   i32,
        surface: &[i32],
        rng:     &mut AreniteRng,
    ) {
        for x in 0..width {
            let sy = surface.get(x as usize).copied().unwrap_or(0);
            if rng.i32_range(0, 30) == 0 {
                let deco = MaterialInstance {
                    id:      0,
                    physics: PhysicsType::Solid,
                    color:   arenite_core::Color::rgb(0, 180, 20),
                    light:   [0.0; 3],
                    data:    0,
                };
                world.set_pixel(TilePos::new(x, sy - 1), deco);
            }
        }
    }
}
