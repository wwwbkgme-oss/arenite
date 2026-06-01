// World save / load — T-022
//
// Layout on disk:
//   saves/<name>/meta.bin          — WorldMeta (tick + seed)
//   saves/<name>/chunks/<cx>_<cy>.bin — per-chunk pixel + light data

use std::fs;
use std::path::Path;

use crate::chunk::ChunkData;
use crate::world::SimWorld;
use arenite_core::error::{AreniteError, AreniteResult};
use arenite_core::pos::ChunkPos;

#[derive(serde::Serialize, serde::Deserialize)]
struct WorldMeta {
    tick: u64,
    seed: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChunkSave {
    pixels: Vec<crate::material::MaterialInstance>,
    light: Vec<[f32; 3]>,
}

/// Persist `world` to `<dir>/<name>/`.
pub fn save_world(world: &SimWorld, dir: &Path, name: &str) -> AreniteResult<()> {
    let root = dir.join(name);
    let chunk_dir = root.join("chunks");
    fs::create_dir_all(&chunk_dir)?;

    let meta_bytes = bincode::serialize(&WorldMeta {
        tick: world.tick,
        seed: world.seed,
    })
    .map_err(|e| AreniteError::Serialise(e.to_string()))?;
    fs::write(root.join("meta.bin"), &meta_bytes)?;

    for (pos, cell) in &world.chunks {
        // SAFETY: called from main thread between ticks.
        let data = unsafe { &*cell.get() };
        let bytes = bincode::serialize(&ChunkSave {
            pixels: data.pixels.clone(),
            light: data.light.clone(),
        })
        .map_err(|e| AreniteError::Serialise(e.to_string()))?;
        fs::write(chunk_dir.join(format!("{}_{}.bin", pos.x, pos.y)), &bytes)?;
    }

    log::info!("Saved '{}' — {} chunks", name, world.chunks.len());
    Ok(())
}

/// Load a world from `<dir>/<name>/`.
pub fn load_world(dir: &Path, name: &str) -> AreniteResult<SimWorld> {
    let root = dir.join(name);

    let meta: WorldMeta = bincode::deserialize(&fs::read(root.join("meta.bin"))?)
        .map_err(|e| AreniteError::Serialise(e.to_string()))?;

    let mut world = SimWorld::new(meta.seed);
    world.tick = meta.tick;

    let chunk_dir = root.join("chunks");
    if chunk_dir.exists() {
        for entry in fs::read_dir(&chunk_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "bin") {
                if let Some(pos) = parse_chunk_filename(path.file_stem()) {
                    let save: ChunkSave = bincode::deserialize(&fs::read(&path)?)
                        .map_err(|e| AreniteError::Serialise(e.to_string()))?;
                    let mut data = ChunkData::new_empty();
                    data.pixels = save.pixels;
                    data.light = save.light;
                    data.dirty.dirty = true; // force GPU re-upload
                    world.insert_chunk(pos, data);
                }
            }
        }
    }

    log::info!(
        "Loaded '{}' — {} chunks, tick={}",
        name,
        world.chunks.len(),
        world.tick
    );
    Ok(world)
}

fn parse_chunk_filename(stem: Option<&std::ffi::OsStr>) -> Option<ChunkPos> {
    let s = stem?.to_str()?;
    let (xs, ys) = s.split_once('_')?;
    Some(ChunkPos::new(xs.parse().ok()?, ys.parse().ok()?))
}
