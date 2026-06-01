// T-039 Integration tests — worldgen + simulation pipeline.
use arenite_world::{WorldGenerator, worldgen::WorldGenConfig};
use arenite_sim::{
    SimWorld, chunk::ChunkData,
    material::{MaterialInstance, flags},
    physics_type::PhysicsType,
};
use arenite_core::pos::{ChunkPos, TilePos};

#[test]
fn integration_generate_and_tick() {
    let cfg = WorldGenConfig { width: 128, height: 64, seed: 99999, sea_level: 32, cave_density: 1.0 };
    let mut sim = WorldGenerator::new(cfg).generate();
    assert!(sim.loaded_chunk_count() > 0);
    let total = sim.total_pixel_count();
    // Drop some water to ensure liquid sim runs.
    let water = MaterialInstance { id: 5, physics: PhysicsType::Liquid, color: arenite_core::Color::WATER, light: [0.0;3], data: 0 };
    for dx in -3..=3i32 { sim.set_pixel(TilePos::new(64 + dx, 10), water); }
    for _ in 0..100 { sim.tick_simulation(); }
    assert_eq!(sim.total_pixel_count(), total);
}

#[test]
fn integration_worldgen_has_surface_tiles() {
    let cfg = WorldGenConfig { width: 64, height: 32, seed: 42, sea_level: 16, cave_density: 0.5 };
    let sim = WorldGenerator::new(cfg).generate();
    let found = (0..32i32).any(|y| sim.get_pixel(TilePos::new(32, y)).physics == PhysicsType::Solid);
    assert!(found, "no solid surface in centre column");
}

#[test]
fn integration_fire_spreads_to_flammable_wood() {
    let mut sim = SimWorld::new(1);
    sim.insert_chunk(ChunkPos::new(0, 0), ChunkData::new_empty());
    let wood = MaterialInstance { id: 12, physics: PhysicsType::Solid, color: arenite_core::Color::WOOD, light: [0.0;3], data: flags::FLAMMABLE };
    let fire = MaterialInstance { id: 9, physics: PhysicsType::Fire, color: arenite_core::Color::new(255,140,0,200), light: [1.0,0.6,0.1], data: 60 };
    sim.set_pixel(TilePos::new(10, 10), wood);
    sim.set_pixel(TilePos::new(11, 10), fire);
    let burned = (0..200).any(|_| {
        sim.tick_simulation();
        let p = sim.get_pixel(TilePos::new(10, 10));
        p.physics == PhysicsType::Fire || p.is_air()
    });
    assert!(burned, "fire did not spread to wood in 200 ticks");
}
