use arenite_core::pos::TilePos;
use arenite_sim::SimWorld;
use arenite_sim::material::MaterialInstance;
use arenite_sim::physics_type::PhysicsType;
use arenite_core::rng::AreniteRng;

/// A 2D stamp of pixels that can be placed at a world position.
/// Inspired by terra-awg's structures/ directory and
/// FallingSandEngine's structure/jigsaw_structure system.
#[derive(Clone, Debug)]
pub struct Structure {
    pub name:   &'static str,
    pub width:  i32,
    pub height: i32,
    /// Row-major pixel data: None = leave untouched, Some = place material.
    pub pixels: Vec<Option<MaterialInstance>>,
}

impl Structure {
    pub fn place(&self, world: &mut SimWorld, origin_x: i32, origin_y: i32) {
        for dy in 0..self.height {
            for dx in 0..self.width {
                let idx = (dy * self.width + dx) as usize;
                if let Some(mat) = self.pixels[idx] {
                    world.set_pixel(TilePos::new(origin_x + dx, origin_y + dy), mat);
                }
            }
        }
    }
}

fn solid(color: arenite_core::Color) -> Option<MaterialInstance> {
    Some(MaterialInstance {
        id: 0,
        physics: PhysicsType::Solid,
        color,
        light: [0.0; 3],
        data: 0,
    })
}

fn air() -> Option<MaterialInstance> {
    Some(MaterialInstance::air())
}

/// Small surface house made of wood.
pub fn make_house() -> Structure {
    use arenite_core::Color;
    let w = Color::WOOD;
    let s = Color::rgb(200, 170, 120); // straw roof

    // 7 wide × 5 tall:
    // SSSSSSS
    // W_____W
    // W_____W
    // W__D__W
    // WWWWWWW
    let pixels: Vec<Option<MaterialInstance>> = vec![
        solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),
        solid(w),air(),  air(),  air(),  air(),  air(),  solid(w),
        solid(w),air(),  air(),  air(),  air(),  air(),  solid(w),
        solid(w),air(),  air(),  solid(w),air(),  air(),  solid(w),
        solid(w),solid(w),solid(w),solid(w),solid(w),solid(w),solid(w),
    ];

    Structure { name: "house", width: 7, height: 5, pixels }
}

/// Desert pyramid outline.
pub fn make_pyramid() -> Structure {
    use arenite_core::Color;
    let s = Color::rgb(210, 185, 110); // sandstone
    let e = None;

    // 9 wide × 5 tall pyramid:
    let row1 = vec![e,e,e,e,solid(s),e,e,e,e];
    let row2 = vec![e,e,e,solid(s),solid(s),solid(s),e,e,e];
    let row3 = vec![e,e,solid(s),solid(s),solid(s),solid(s),solid(s),e,e];
    let row4 = vec![e,solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),e];
    let row5 = vec![solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),solid(s),solid(s)];

    let pixels: Vec<Option<MaterialInstance>> = row1.into_iter()
        .chain(row2).chain(row3).chain(row4).chain(row5)
        .collect();

    Structure { name: "pyramid", width: 9, height: 5, pixels }
}

/// Place structures at appropriate surface positions.
pub fn place_structures(
    world:   &mut SimWorld,
    width:   i32,
    surface: &[i32],
    rng:     &mut AreniteRng,
) {
    let house   = make_house();
    let pyramid = make_pyramid();

    // Scatter some houses.
    let house_count = width / 300;
    for _ in 0..house_count {
        let x = rng.i32_range(10, width - 20);
        let y = surface.get(x as usize).copied().unwrap_or(0);
        house.place(world, x, y - house.height);
    }

    // Place a couple of pyramids in desert-ish areas.
    let pyramid_count = width / 800;
    for _ in 0..pyramid_count {
        let x = rng.i32_range(50, width - 60);
        let y = surface.get(x as usize).copied().unwrap_or(0);
        pyramid.place(world, x, y - pyramid.height);
    }
}
