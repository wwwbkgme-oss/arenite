use arenite_core::pos::TilePos;
use arenite_core::rng::AreniteRng;
use arenite_sim::material::{flags, MaterialInstance};
use arenite_sim::physics_type::PhysicsType;
use arenite_sim::SimWorld;

/// A 2D stamp of pixels that can be placed at a world position.
/// Inspired by terra-awg's structures/ directory and
/// FallingSandEngine's structure/jigsaw_structure system.
#[derive(Clone, Debug)]
pub struct Structure {
    pub name: &'static str,
    pub width: i32,
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

// ── Pixel helpers ────────────────────────────────────────────────────────────

fn solid(color: arenite_core::Color) -> Option<MaterialInstance> {
    Some(MaterialInstance {
        id: 0,
        physics: PhysicsType::Solid,
        color,
        light: [0.0; 3],
        data: 0,
    })
}

/// Like `solid` but marks the pixel combustible (T-035).
fn flammable_solid(color: arenite_core::Color) -> Option<MaterialInstance> {
    Some(MaterialInstance {
        id: 0,
        physics: PhysicsType::Solid,
        color,
        light: [0.0; 3],
        data: flags::FLAMMABLE,
    })
}

/// Emissive solid (crystal, mushroom caps).
fn emissive_solid(color: arenite_core::Color, emission: [f32; 3]) -> Option<MaterialInstance> {
    Some(MaterialInstance {
        id: 0,
        physics: PhysicsType::Solid,
        color,
        light: emission,
        data: 0,
    })
}

fn air() -> Option<MaterialInstance> {
    Some(MaterialInstance::air())
}

// ── Structures ────────────────────────────────────────────────────────────────

/// Small surface house made of wood.
pub fn make_house() -> Structure {
    use arenite_core::Color;
    let w = Color::WOOD;
    let s = Color::rgb(200, 170, 120); // straw roof (flammable)

    // 7 wide × 5 tall:
    // SSSSSSS
    // W_____W
    // W_____W
    // W__D__W
    // WWWWWWW
    // All wood and straw are marked flammable (T-035).
    let pixels: Vec<Option<MaterialInstance>> = vec![
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(s),
        flammable_solid(w),
        air(),
        air(),
        air(),
        air(),
        air(),
        flammable_solid(w),
        flammable_solid(w),
        air(),
        air(),
        air(),
        air(),
        air(),
        flammable_solid(w),
        flammable_solid(w),
        air(),
        air(),
        flammable_solid(w),
        air(),
        air(),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
        flammable_solid(w),
    ];

    Structure {
        name: "house",
        width: 7,
        height: 5,
        pixels,
    }
}

/// Desert pyramid outline.
pub fn make_pyramid() -> Structure {
    use arenite_core::Color;
    let s = Color::rgb(210, 185, 110); // sandstone
    let e = None;

    // 9 wide × 5 tall pyramid:
    let row1 = vec![e, e, e, e, solid(s), e, e, e, e];
    let row2 = vec![e, e, e, solid(s), solid(s), solid(s), e, e, e];
    let row3 = vec![e, e, solid(s), solid(s), solid(s), solid(s), solid(s), e, e];
    let row4 = vec![
        e,
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        e,
    ];
    let row5 = vec![
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
        solid(s),
    ];

    let pixels: Vec<Option<MaterialInstance>> = row1
        .into_iter()
        .chain(row2)
        .chain(row3)
        .chain(row4)
        .chain(row5)
        .collect();

    Structure {
        name: "pyramid",
        width: 9,
        height: 5,
        pixels,
    }
}

/// Small tree: 3 wide × 6 tall (trunk + round canopy).
/// Inspired by re-flora's flora system and Terraria's tree shapes.
///
/// ```text
/// _L_
/// LLL   ← canopy rows (3 wide × 3 tall)
/// LLL
/// _T_   ← trunk
/// _T_
/// _T_   ← base (placed at surface)
/// ```
pub fn make_tree_small() -> Structure {
    use arenite_core::Color;
    let t = Color::WOOD;
    let l = Color::rgb(34, 139, 34); // forest green leaves
    let e = None;

    let pixels: Vec<Option<MaterialInstance>> = vec![
        // Top canopy row (3 wide)
        e,
        flammable_solid(l),
        e,
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        // Trunk (centre column, 3 rows)
        e,
        flammable_solid(t),
        e,
        e,
        flammable_solid(t),
        e,
        e,
        flammable_solid(t),
        e,
    ];

    Structure {
        name: "tree_small",
        width: 3,
        height: 6,
        pixels,
    }
}

/// Large tree: 5 wide × 8 tall.
///
/// ```text
/// __L__
/// _LLL_
/// LLLLL
/// LLLLL   ← canopy (5 wide × 4 tall)
/// __T__
/// __T__
/// __T__   ← trunk
/// __T__
/// ```
pub fn make_tree_large() -> Structure {
    use arenite_core::Color;
    let t = Color::WOOD;
    let l = Color::rgb(28, 120, 28); // slightly darker green for large trees
    let e = None;

    let pixels: Vec<Option<MaterialInstance>> = vec![
        // Row 0: top of canopy
        e,
        e,
        flammable_solid(l),
        e,
        e,
        // Row 1
        e,
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        e,
        // Row 2
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        // Row 3
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        // Trunk rows (4 rows)
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
    ];

    Structure {
        name: "tree_large",
        width: 5,
        height: 8,
        pixels,
    }
}

/// Jungle tree: 5 wide × 9 tall with vines.
pub fn make_tree_jungle() -> Structure {
    use arenite_core::Color;
    let t = Color::rgb(100, 60, 20); // darker tropical wood
    let l = Color::rgb(0, 160, 30); // bright jungle green
    let v = Color::rgb(0, 140, 10); // vine (hanging leaf)
    let e = None;

    let pixels: Vec<Option<MaterialInstance>> = vec![
        // Canopy (5 wide × 4 tall)
        e,
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        e,
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(v),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(v),
        // Trunk (5 rows)
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
        e,
        e,
        flammable_solid(t),
        e,
        e,
    ];

    Structure {
        name: "tree_jungle",
        width: 5,
        height: 9,
        pixels,
    }
}

/// Tundra pine tree: narrow 3-wide × 7-tall spruce shape.
pub fn make_tree_pine() -> Structure {
    use arenite_core::Color;
    let t = Color::rgb(80, 50, 20); // dark bark
    let l = Color::rgb(20, 100, 40); // dark pine-green needles
    let e = None;

    let pixels: Vec<Option<MaterialInstance>> = vec![
        // Tip
        e,
        flammable_solid(l),
        e,
        // Upper canopy widening rows
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        flammable_solid(l),
        // Trunk (3 rows)
        e,
        flammable_solid(t),
        e,
        e,
        flammable_solid(t),
        e,
        e,
        flammable_solid(t),
        e,
    ];

    Structure {
        name: "tree_pine",
        width: 3,
        height: 7,
        pixels,
    }
}

/// Mushroom: tall stem + wide cap for underground mushroom biomes.
/// Inspired by Terraria's glowing mushroom biome and Starbound's fungal caves.
pub fn make_mushroom_large() -> Structure {
    let c = arenite_core::Color::MUSHROOM; // cap
    let s = arenite_core::Color::rgb(200, 200, 210); // white stem
    let e = None;

    // 5 wide × 5 tall
    let pixels: Vec<Option<MaterialInstance>> = vec![
        // Cap (5 wide)
        emissive_solid(c, [0.08, 0.0, 0.15]),
        emissive_solid(c, [0.08, 0.0, 0.15]),
        emissive_solid(c, [0.08, 0.0, 0.15]),
        emissive_solid(c, [0.08, 0.0, 0.15]),
        emissive_solid(c, [0.08, 0.0, 0.15]),
        // Cap underside
        e,
        emissive_solid(c, [0.05, 0.0, 0.1]),
        emissive_solid(c, [0.05, 0.0, 0.1]),
        emissive_solid(c, [0.05, 0.0, 0.1]),
        e,
        // Stem (2 rows, centre column)
        e,
        e,
        solid(s),
        e,
        e,
        e,
        e,
        solid(s),
        e,
        e,
        e,
        e,
        solid(s),
        e,
        e,
    ];

    Structure {
        name: "mushroom_large",
        width: 5,
        height: 5,
        pixels,
    }
}

/// Crystal spike cluster for cave biomes (Starbound crystal caves).
pub fn make_crystal_spike() -> Structure {
    let c = arenite_core::Color::CRYSTAL;
    let bright = [0.0_f32, 0.7, 1.0];
    let dim = [0.0_f32, 0.4, 0.6];
    let e = None;

    // 3 wide × 4 tall — upward-pointing spikes
    let pixels: Vec<Option<MaterialInstance>> = vec![
        e,
        emissive_solid(c, bright),
        e,
        emissive_solid(c, dim),
        emissive_solid(c, bright),
        emissive_solid(c, dim),
        emissive_solid(c, dim),
        emissive_solid(c, bright),
        emissive_solid(c, dim),
        emissive_solid(c, dim),
        emissive_solid(c, dim),
        emissive_solid(c, dim),
    ];

    Structure {
        name: "crystal_spike",
        width: 3,
        height: 4,
        pixels,
    }
}

// ── Placement ────────────────────────────────────────────────────────────────

/// Place all surface structures: houses, pyramids, and biome-appropriate trees.
pub fn place_structures(
    world: &mut SimWorld,
    width: i32,
    surface: &[i32],
    rng: &mut AreniteRng,
    biome_ids: &[u8], // per-column biome ID (0=plains, 1=desert, 2=tundra, 3=jungle…)
) {
    let house = make_house();
    let pyramid = make_pyramid();

    // Scatter some houses on plains/jungle.
    let house_count = width / 300;
    for _ in 0..house_count {
        let x = rng.i32_range(10, width - 20);
        let biome = biome_ids.get(x as usize).copied().unwrap_or(0);
        if biome == 0 || biome == 3 {
            let y = surface.get(x as usize).copied().unwrap_or(0);
            house.place(world, x, y - house.height);
        }
    }

    // Place pyramids in desert areas.
    let pyramid_count = width / 800;
    for _ in 0..pyramid_count {
        let x = rng.i32_range(50, width - 60);
        let biome = biome_ids.get(x as usize).copied().unwrap_or(0);
        if biome == 1 {
            let y = surface.get(x as usize).copied().unwrap_or(0);
            pyramid.place(world, x, y - pyramid.height);
        }
    }

    // Trees — density and type by biome.
    place_trees(world, width, surface, rng, biome_ids);
}

/// Scatter trees across the surface, choosing variety by biome.
/// Inspired by re-flora's flora species placement and Terraria's tree generation.
pub fn place_trees(
    world: &mut SimWorld,
    width: i32,
    surface: &[i32],
    rng: &mut AreniteRng,
    biome_ids: &[u8],
) {
    let tree_small = make_tree_small();
    let tree_large = make_tree_large();
    let tree_jungle = make_tree_jungle();
    let tree_pine = make_tree_pine();

    // One tree attempt every ~12 tiles.
    let attempts = width / 12;
    for _ in 0..attempts {
        let x = rng.i32_range(6, width - 8);
        let biome = biome_ids.get(x as usize).copied().unwrap_or(0);
        let sy = surface.get(x as usize).copied().unwrap_or(0);

        // Skip ocean edges and desert.
        if biome == 4 || biome == 1 {
            continue;
        }

        // Choose tree type by biome.
        let (tree, offset_x) = match biome {
            3 => (&tree_jungle, 2), // jungle tree, centre offset
            2 => (&tree_pine, 1),   // tundra pine
            _ => {
                if rng.i32_range(0, 3) == 0 {
                    (&tree_large, 2)
                } else {
                    (&tree_small, 1)
                }
            }
        };

        // Place tree so its base aligns with the surface.
        let place_x = x - offset_x;
        let place_y = sy - tree.height;
        tree.place(world, place_x, place_y);
    }
}

/// Place mushroom decorations on underground mud/mushroom floors.
pub fn place_mushrooms(
    world: &mut SimWorld,
    width: i32,
    underground_y: i32,
    cavern_y: i32,
    rng: &mut AreniteRng,
) {
    let mushroom = make_mushroom_large();
    let count = width / 60;
    for _ in 0..count {
        let x = rng.i32_range(4, width - 6);
        let y = rng.i32_range(underground_y, cavern_y.min(underground_y + 300));
        // Only place on solid ground.
        let below = world.get_pixel(arenite_core::pos::TilePos::new(x, y + 1));
        if !below.is_air() && below.physics == PhysicsType::Solid {
            mushroom.place(world, x - 2, y - mushroom.height + 1);
        }
    }
}

/// Place crystal spike clusters deep in cave walls.
pub fn place_crystal_clusters(
    world: &mut SimWorld,
    width: i32,
    cavern_y: i32,
    world_h: i32,
    rng: &mut AreniteRng,
) {
    let spike = make_crystal_spike();
    let count = width / 40;
    for _ in 0..count {
        let x = rng.i32_range(4, width - 5);
        let y = rng.i32_range(cavern_y, (world_h - 50).max(cavern_y + 10));
        // Place on solid floor.
        let floor = world.get_pixel(arenite_core::pos::TilePos::new(x, y + 1));
        if floor.physics == PhysicsType::Solid && !floor.is_air() {
            spike.place(world, x - 1, y - spike.height + 1);
        }
    }
}

/// Generate floating sky islands (Terraria sky islands / Starbound sky biome).
///
/// Each island is a filled ellipse of stone topped with a grass surface.
/// Islands are placed at high altitude (above 10% of world height).
pub fn place_floating_islands(world: &mut SimWorld, width: i32, height: i32, rng: &mut AreniteRng) {
    let island_count = (width / 500).max(2).min(6);
    let ceiling = height / 12; // maximum y for island centres

    for _ in 0..island_count {
        let cx = rng.i32_range(40, width - 40);
        let cy = rng.i32_range(10, ceiling.max(11));
        let rx = rng.i32_range(18, 35); // horizontal radius
        let ry = rng.i32_range(6, 14); // vertical radius (flatter shape)

        // Fill ellipse with stone.
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                // Ellipse equation: (dx/rx)^2 + (dy/ry)^2 <= 1
                let ex = dx as f32 / rx as f32;
                let ey = dy as f32 / ry as f32;
                if ex * ex + ey * ey > 1.0 {
                    continue;
                }

                let wx = cx + dx;
                let wy = cy + dy;
                if wx < 0 || wx >= width || wy < 0 || wy >= height {
                    continue;
                }

                let tp = arenite_core::pos::TilePos::new(wx, wy);
                let mat = MaterialInstance {
                    id: 0,
                    physics: PhysicsType::Solid,
                    color: if dy < -ry + 2 {
                        arenite_core::Color::GRASS // thin grass cap on top
                    } else {
                        arenite_core::Color::STONE
                    },
                    light: [0.0; 3],
                    data: if dy < -ry + 2 { flags::FLAMMABLE } else { 0 },
                };
                world.set_pixel(tp, mat);
            }
        }

        // Optional: a small cloud of lighter stone/dirt directly beneath.
        let tail_len = ry / 2;
        for dy in 1..=tail_len {
            let tw = (rx as f32 * (1.0 - dy as f32 / tail_len as f32)) as i32;
            for dx in -tw..=tw {
                let tp = arenite_core::pos::TilePos::new(cx + dx, cy + ry + dy);
                if world.get_pixel(tp).is_air() {
                    world.set_pixel(
                        tp,
                        MaterialInstance {
                            id: 0,
                            physics: PhysicsType::Solid,
                            color: arenite_core::Color::DIRT,
                            light: [0.0; 3],
                            data: 0,
                        },
                    );
                }
            }
        }
    }
}
