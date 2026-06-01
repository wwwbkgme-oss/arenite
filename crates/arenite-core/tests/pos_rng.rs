use arenite_core::pos::{TilePos, ChunkPos, CHUNK_SIZE};
use arenite_core::rng::AreniteRng;

#[test]
fn tile_to_chunk_positive() {
    assert_eq!(TilePos::new(65, 130).to_chunk(), ChunkPos::new(1, 2));
}

#[test]
fn tile_to_chunk_negative() {
    let tp = TilePos::new(-1, -1);
    assert_eq!(tp.to_chunk(), ChunkPos::new(-1, -1));
    let (lx, ly) = tp.local();
    assert_eq!((lx, ly), (CHUNK_SIZE - 1, CHUNK_SIZE - 1));
}

#[test]
fn chunk_origin_tile() {
    let cp = ChunkPos::new(3, 5);
    assert_eq!(cp.origin_tile(), TilePos::new(192, 320));
}

#[test]
fn tile_local_index_corners() {
    assert_eq!(TilePos::new(0, 0).local_index(), 0);
    let last = (CHUNK_SIZE * CHUNK_SIZE - 1) as usize;
    assert_eq!(TilePos::new(CHUNK_SIZE - 1, CHUNK_SIZE - 1).local_index(), last);
}

#[test]
fn rng_deterministic() {
    let mut a = AreniteRng::from_seed(99);
    let mut b = AreniteRng::from_seed(99);
    for _ in 0..200 { assert_eq!(a.u64(), b.u64()); }
}

#[test]
fn rng_coords_differ() {
    let a = AreniteRng::from_coords(0, 0, 0).u64();
    let b = AreniteRng::from_coords(1, 0, 0).u64();
    let c = AreniteRng::from_coords(0, 1, 0).u64();
    assert_ne!(a, b); assert_ne!(a, c); assert_ne!(b, c);
}

#[test]
fn rng_i32_range() {
    let mut rng = AreniteRng::from_seed(7);
    for _ in 0..1000 {
        let v = rng.i32_range(5, 10);
        assert!((5..10).contains(&v), "got {v}");
    }
}
