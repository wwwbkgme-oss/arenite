use arenite_core::pos::{ChunkPos, TilePos, CHUNK_SIZE};
use arenite_sim::{
    chunk::ChunkData, material::MaterialInstance, physics_type::PhysicsType, SimWorld,
};

fn solid_mat() -> MaterialInstance {
    MaterialInstance {
        id: 2,
        physics: PhysicsType::Solid,
        color: arenite_core::Color::STONE,
        light: [0.0; 3],
        data: 0,
    }
}
fn sand_mat() -> MaterialInstance {
    MaterialInstance {
        id: 3,
        physics: PhysicsType::Sand,
        color: arenite_core::Color::SAND,
        light: [0.0; 3],
        data: 0,
    }
}
fn water_mat() -> MaterialInstance {
    MaterialInstance {
        id: 5,
        physics: PhysicsType::Liquid,
        color: arenite_core::Color::WATER,
        light: [0.0; 3],
        data: 0,
    }
}

fn one_chunk_world(data: ChunkData) -> SimWorld {
    let mut w = SimWorld::new(0);
    w.insert_chunk(ChunkPos::new(0, 0), data);
    w
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[test]
fn world_loaded_count() {
    let mut w = SimWorld::new(0);
    for cx in 0..3 {
        for cy in 0..3 {
            w.insert_chunk(ChunkPos::new(cx, cy), ChunkData::new_empty());
        }
    }
    assert_eq!(w.loaded_chunk_count(), 9);
}

#[test]
fn set_get_pixel() {
    let mut w = SimWorld::new(0);
    w.insert_chunk(ChunkPos::new(0, 0), ChunkData::new_empty());
    let tp = TilePos::new(5, 7);
    w.set_pixel(tp, sand_mat());
    assert_eq!(w.get_pixel(tp).physics, PhysicsType::Sand);
}

#[test]
fn out_of_bounds_pixel_is_air() {
    let w = SimWorld::new(0);
    assert!(w.get_pixel(TilePos::new(999, 999)).is_air());
}

// ── sand physics ──────────────────────────────────────────────────────────────

#[test]
fn sand_falls_one_tile() {
    let mut data = ChunkData::new_empty();
    data.set(10, 10, sand_mat());
    let mut w = one_chunk_world(data);
    w.tick_simulation();
    assert!(
        w.get_pixel(TilePos::new(10, 10)).is_air(),
        "sand should leave (10,10)"
    );
    assert!(
        !w.get_pixel(TilePos::new(10, 11)).is_air(),
        "sand should be at (10,11)"
    );
}

#[test]
fn sand_rests_on_solid() {
    let mut data = ChunkData::new_empty();
    // Solid floor at y=20.
    for x in 0..CHUNK_SIZE {
        data.set(x, 20, solid_mat());
    }
    data.set(10, 5, sand_mat());
    let mut w = one_chunk_world(data);
    for _ in 0..25 {
        w.tick_simulation();
    }
    assert_eq!(w.get_pixel(TilePos::new(10, 19)).physics, PhysicsType::Sand);
}

// ── liquid physics ────────────────────────────────────────────────────────────

#[test]
fn water_falls() {
    let mut data = ChunkData::new_empty();
    // Solid floor at y=30.
    for x in 0..CHUNK_SIZE {
        data.set(x, 30, solid_mat());
    }
    data.set(20, 5, water_mat());
    let mut w = one_chunk_world(data);
    for _ in 0..80 {
        w.tick_simulation();
    }
    // The water should no longer be at its starting position.
    let still_at_start = w.get_pixel(TilePos::new(20, 5)).physics == PhysicsType::Liquid;
    assert!(
        !still_at_start,
        "water should have moved from (20,5) after 80 ticks"
    );
}

// ── save / load ───────────────────────────────────────────────────────────────

#[test]
fn save_load_roundtrip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut w = SimWorld::new(42);
    let mut data = ChunkData::new_empty();
    data.set(3, 7, sand_mat());
    w.insert_chunk(ChunkPos::new(0, 0), data);
    w.tick = 100;

    arenite_sim::save_world(&w, tmp.path(), "test").expect("save");
    let w2 = arenite_sim::load_world(tmp.path(), "test").expect("load");

    assert_eq!(w2.tick, 100);
    assert_eq!(w2.seed, 42);
    assert_eq!(w2.get_pixel(TilePos::new(3, 7)).physics, PhysicsType::Sand);
}
