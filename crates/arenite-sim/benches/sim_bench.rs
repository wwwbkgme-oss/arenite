// T-040: Criterion benchmarks for the simulation engine.
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use arenite_sim::{SimWorld, chunk::ChunkData, material::MaterialInstance, physics_type::PhysicsType};
use arenite_core::pos::ChunkPos;

fn make_sim(n: i32, with_sand: bool) -> SimWorld {
    let mut sim = SimWorld::new(0);
    for cy in 0..n { for cx in 0..n {
        let mut d = ChunkData::new_empty();
        if with_sand {
            for y in 0..64i32 { for x in 0..64i32 {
                if y == 60 {
                    d.set(x, y, MaterialInstance { id:2, physics: PhysicsType::Solid, color: arenite_core::Color::STONE, light:[0.0;3], data:0 });
                } else if (x + y*7) % 17 == 0 {
                    d.set(x, y, MaterialInstance { id:3, physics: PhysicsType::Sand, color: arenite_core::Color::SAND, light:[0.0;3], data:0 });
                }
            }}
        } else {
            for y in 0..64i32 { for x in 0..64i32 {
                d.set(x, y, MaterialInstance { id:2, physics: PhysicsType::Solid, color: arenite_core::Color::STONE, light:[0.0;3], data:0 });
            }}
        }
        sim.insert_chunk(ChunkPos::new(cx, cy), d);
    }}
    sim
}

fn bench_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("sim");
    for n in [4i32, 8] {
        let mut sim = make_sim(n, true);
        g.bench_with_input(BenchmarkId::new("tick_dynamic", n*n), &n, |b, _| b.iter(|| sim.tick_simulation()));
    }
    let mut s = make_sim(8, false);
    g.bench_function("tick_all_static_64chunks", |b| b.iter(|| s.tick_simulation()));
    g.finish();
}

criterion_group!(benches, bench_tick);
criterion_main!(benches);
