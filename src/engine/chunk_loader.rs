use crate::{
    engine::{
        Chunk, Material, Renderer,
        marching_cubes::{self, Mesh},
        thread_pool::ThreadPool,
        world_gen,
    },
    prelude::*,
};
use anyhow::Result;
use bincode_next::config::{self};
use rusqlite::{Connection, params};
use std::{
    alloc,
    collections::{HashMap, HashSet},
    num::NonZero,
    ptr,
};

pub struct ChunkLoader {
    // TODO: generating chunks
    generated_chunks: HashSet<(i32, i32, i32)>,
    loaded_chunks: HashMap<(i32, i32, i32), Chunk>,
    settings: ChunkLoaderSettings,
    db: Connection,
    generator: ThreadPool<Chunk>,
    mesh_builder: ThreadPool<Mesh>,
}

pub struct ChunkLoaderSettings {
    pub view_distance: i32,
    pub thread_count: NonZero<usize>,
    pub db_path: String,
    pub world_height: i32,
}

impl ChunkLoader {
    pub fn new(settings: ChunkLoaderSettings) -> Self {
        let db = Connection::open(&settings.db_path).expect("could not open database");
        Chunk::create_table(&db).expect("could not create chunk table");
        create_generated_chunks_table(&db).expect("could not create generated chunks table");

        let mut generated_chunks = HashSet::new();
        while let Some(row) = db
            .prepare("SELECT x, y, z FROM chunks")
            .unwrap()
            .query(params![])
            .expect("could not read generated chunks")
            .next()
            .unwrap()
        {
            let x: i32 = row.get(0).unwrap();
            let y: i32 = row.get(1).unwrap();
            let z: i32 = row.get(2).unwrap();

            generated_chunks.insert((x, y, z));
        }

        Self {
            generated_chunks,
            loaded_chunks: HashMap::new(),
            db,
            generator: ThreadPool::new(settings.thread_count.get()),
            mesh_builder: ThreadPool::new(settings.thread_count.get()),
            settings,
        }
    }

    pub fn update(&mut self, renderer: &mut Renderer, cam_x: i32, cam_y: i32) {
        let view_dist = self.settings.view_distance;

        let div = |a: i32, b: i32| {
            let div = a / b;
            if div < 0 { div - 1 } else { div }
        };

        let cam_chunk_x = div(cam_x, Chunk::WIDTH as i32);
        let cam_chunk_y = div(cam_y, Chunk::WIDTH as i32);

        // Look for chunks to generate or load
        for x in (-view_dist..view_dist).map(|i| i + cam_chunk_x) {
            for y in (-view_dist..view_dist).map(|i| i + cam_chunk_y) {
                for z in 0..self.settings.world_height {
                    if !self.generated_chunks.contains(&(x, y, z)) {
                        self.generate_chunk(x, y, z);
                        continue;
                    }
                    if !self.loaded_chunks.contains_key(&(x, y, z)) {
                        let chunk = Chunk::find(&self.db, x, y, z)
                            .unwrap()
                            .expect("could not load chunk, even though it is marked as generated");
                        self.loaded_chunks.insert((x, y, z), chunk);
                    }
                }
            }
        }

        // Unload chunks that are no longer needed
        self.loaded_chunks
            .retain_filter(|chunk| {
                let is_in_range = (chunk.pos[0] - cam_chunk_x).abs() <= view_dist
                    && (chunk.pos[1] - cam_chunk_y).abs() <= view_dist;

                is_in_range
            })
            .into_iter()
            .for_each(|chunk| {
                chunk.insert(&self.db).unwrap();
            });

        let task_count = self.generator.task_count();
        if task_count > 0 {
            println!("world gen task: {task_count}");
        }

        // Receive generated chunks from the thread pool
        // And invoke the mesh builder
        //
        // for result in self.generator.results().into_iter() {
        //     let [x, y, z] = result.pos;
        //     self.loaded_chunks.insert((x, y, z), result);
        //     self.mesh_builder.add_task(move || {
        //         let mut voxels = marching_cubes::Voxels::new();
        //         voxels.insert_chunk(result);
        //         voxels.get_chunk_mesh([x, y, z])
        //     });
        // }
    }

    pub fn get_chunks(&mut self) -> Vec<&mut Chunk> {
        self.loaded_chunks.values_mut().collect()
    }

    fn generate_chunk(&mut self, x: i32, y: i32, z: i32) {
        self.generator
            .add_task(move || world_gen::generate_chunk(x, y, z));
    }
}

impl Chunk {
    fn create_table(db: &Connection) -> Result<()> {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS chunks (
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                z INTEGER NOT NULL,
                materials BLOB NOT NULL,
                amount BLOB NOT NULL,
                PRIMARY KEY (x, y, z)
            );",
        )?;
        Ok(())
    }

    fn insert(&self, db: &Connection) -> Result<()> {
        let materials: Vec<u8> = bincode_next::encode_to_vec(
            unsafe { box_to_vec(&self.materials) },
            config::standard(),
        )?;
        let amount: Vec<u8> =
            bincode_next::encode_to_vec(unsafe { box_to_vec(&self.amount) }, config::standard())?;

        db.execute(
            "INSERT OR REPLACE INTO chunks (x, y, z, materials, amount) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![self.pos[0], self.pos[1], self.pos[2], &materials, &amount],
        )?;

        db.execute(
            "INSERT OR REPLACE INTO genchunks (x, y, z) VALUES (?1, ?2, ?3)",
            params![self.pos[0], self.pos[1], self.pos[2]],
        )?;

        Ok(())
    }

    fn find(db: &Connection, x: i32, y: i32, z: i32) -> Result<Option<Self>> {
        let mut query =
            db.prepare("SELECT materials, amount FROM chunks WHERE x = ?1 AND y = ?2 AND z = ?3")?;
        let mut rows = query.query(params![x, y, z])?;

        if let Some(chunk) = rows.next()? {
            let materials: Vec<u8> = chunk.get(0)?;
            let amount: Vec<u8> = chunk.get(1)?;

            let materials: Vec<Material> =
                bincode_next::decode_from_slice(&materials, config::standard())?.0;
            let amount: Vec<f32> = bincode_next::decode_from_slice(&amount, config::standard())?.0;

            Ok(Some(Chunk {
                pos: [x, y, z],
                materials: unsafe { vec_to_box(materials) },
                amount: unsafe { vec_to_box(amount) },
            }))
        } else {
            Ok(None)
        }
    }
}

fn create_generated_chunks_table(db: &Connection) -> anyhow::Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS genchunks (
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            z INTEGER NOT NULL,
            PRIMARY KEY (x, y, z)
        );",
    )?;

    Ok(())
}

fn is_chunk_generated(db: &Connection, x: i32, y: i32, z: i32) -> anyhow::Result<bool> {
    let mut query = db.prepare("SELECT 1 FROM genchunks WHERE x = ?1 AND y = ?2 AND z = ?3")?;
    let mut rows = query.query(params![x, y, z])?;

    Ok(rows.next()?.is_some())
}

unsafe fn vec_to_box<T>(v: Vec<T>) -> Box<[[[T; Chunk::WIDTH]; Chunk::WIDTH]; Chunk::WIDTH]> {
    assert_eq!(v.len(), Chunk::WIDTH * Chunk::WIDTH * Chunk::WIDTH);
    let layout = alloc::Layout::new::<[[[T; Chunk::WIDTH]; Chunk::WIDTH]; Chunk::WIDTH]>();
    let ptr =
        unsafe { alloc::alloc(layout) as *mut [[[T; Chunk::WIDTH]; Chunk::WIDTH]; Chunk::WIDTH] };
    if ptr.is_null() {
        alloc::handle_alloc_error(layout);
    }
    unsafe {
        let dst = ptr as *mut T;
        ptr::copy_nonoverlapping(v.as_ptr(), dst, v.len());
        Box::from_raw(ptr)
    }
}

unsafe fn box_to_vec<T: Copy>(
    b: &Box<[[[T; Chunk::WIDTH]; Chunk::WIDTH]; Chunk::WIDTH]>,
) -> Vec<T> {
    let total = Chunk::WIDTH * Chunk::WIDTH * Chunk::WIDTH;
    let src = &**b as *const [[[T; Chunk::WIDTH]; Chunk::WIDTH]; Chunk::WIDTH] as *const T;
    let mut v = Vec::with_capacity(total);
    unsafe {
        v.set_len(total);
        ptr::copy_nonoverlapping(src, v.as_mut_ptr(), total);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Material;
    use anyhow::Result;
    use bincode_next::config::standard;
    use rusqlite::Connection;

    fn setup_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        Chunk::create_table(&conn)?;
        create_generated_chunks_table(&conn)?;
        Ok(conn)
    }

    #[test]
    fn test_create_tables() -> Result<()> {
        let conn = setup_db()?;
        let count: i32 = conn.query_row(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name IN ('chunks','genchunks')",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 2);
        Ok(())
    }

    #[test]
    fn test_insert_and_find_chunk() -> Result<()> {
        let conn = setup_db()?;
        let chunk = Chunk::stone_block(1, 2, 3);

        chunk.insert(&conn)?;

        assert!(is_chunk_generated(
            &conn,
            chunk.pos[0],
            chunk.pos[1],
            chunk.pos[2]
        )?);

        let found = Chunk::find(&conn, chunk.pos[0], chunk.pos[1], chunk.pos[2])?
            .expect("Chunk should be found");

        assert_eq!(found.pos, chunk.pos);

        assert_eq!(found.materials[0][0][0], chunk.materials[0][0][0]);
        assert_eq!(
            found.materials[1 % Chunk::WIDTH][0][0],
            chunk.materials[1 % Chunk::WIDTH][0][0]
        );
        assert!((found.amount[0][0][0] - chunk.amount[0][0][0]).abs() < f32::EPSILON);
        assert!(
            (found.amount[1 % Chunk::WIDTH][0][0] - chunk.amount[1 % Chunk::WIDTH][0][0]).abs()
                < f32::EPSILON
        );

        Ok(())
    }

    #[test]
    fn test_find_nonexistent_chunk() -> Result<()> {
        let conn = setup_db()?;
        let got = Chunk::find(&conn, 1, 1, 1)?;
        assert!(got.is_none());
        Ok(())
    }

    #[test]
    fn test_update_chunk_serialization_roundtrip() -> Result<()> {
        let conn = setup_db()?;
        let mut chunk = Chunk::stone_block(1, 2, 3);
        chunk.insert(&conn)?;

        chunk.materials[0][0][0] = Material::Sand;
        chunk.amount[0][0][0] = 0.0;

        let materials_bytes =
            bincode_next::encode_to_vec(unsafe { box_to_vec(&chunk.materials) }, standard())?;
        let amount_bytes =
            bincode_next::encode_to_vec(unsafe { box_to_vec(&chunk.amount) }, standard())?;
        conn.execute(
            "UPDATE chunks SET materials = ?1, amount = ?2 WHERE x = ?3 AND y = ?4 AND z = ?5",
            rusqlite::params![
                &materials_bytes,
                &amount_bytes,
                chunk.pos[0],
                chunk.pos[1],
                chunk.pos[2]
            ],
        )?;

        let reloaded = Chunk::find(&conn, chunk.pos[0], chunk.pos[1], chunk.pos[2])?
            .expect("Updated chunk should exist");

        assert_eq!(reloaded.materials[0][0][0], chunk.materials[0][0][0]);
        assert!((reloaded.amount[0][0][0] - chunk.amount[0][0][0]).abs() < f32::EPSILON);

        Ok(())
    }

    #[test]
    fn test_is_chunk_generated_false() -> Result<()> {
        let conn = setup_db()?;
        assert!(!is_chunk_generated(&conn, 0, 0, 0)?);
        Ok(())
    }

    #[test]
    fn test_multiple_inserts_and_replace() -> Result<()> {
        let conn = setup_db()?;

        let mut c1 = Chunk::stone_block(0, 0, 0);
        c1.materials[0][0][0] = Material::Stone;
        c1.amount[0][0][0] = 1.0;
        c1.insert(&conn)?;

        let mut c2 = Chunk::stone_block(0, 0, 0);
        c2.materials[0][0][0] = Material::Dirt;
        c2.amount[0][0][0] = 0.25;
        c2.insert(&conn)?;

        let found = Chunk::find(&conn, 0, 0, 0)?.expect("should exist");
        assert_eq!(found.materials[0][0][0], c2.materials[0][0][0]);
        assert!((found.amount[0][0][0] - c2.amount[0][0][0]).abs() < f32::EPSILON);

        Ok(())
    }
}
