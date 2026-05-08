use crate::{
    engine::{
        Chunk, Material, Renderer,
        marching_cubes::{self, Mesh},
        renderer::{ObjectDataID, prelude::InstanceData},
        thread_pool::ThreadPool,
        world_gen,
    },
    game_object::{GameObjectID, GameObjectTrait},
    prelude::*,
};
use anyhow::Result;
use bincode_next::config::{self};
use cgmath::{Deg, Quaternion, Rad, Rotation3};
use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
use rusqlite::{Connection, params};
use std::{
    alloc,
    collections::{HashMap, HashSet},
    num::NonZero,
    ptr, thread,
};

enum DbCommand {
    Insert(Chunk),
    Find {
        x: i32,
        y: i32,
        z: i32,
        response_tx: Sender<(i32, i32, i32, Option<Chunk>)>,
    },
}

struct DbWorker {
    cmd_tx: Sender<DbCommand>,
}

impl DbWorker {
    fn spawn(db_path: &str, gen_new_world: bool) -> (Self, HashSet<(i32, i32, i32)>) {
        let (cmd_tx, cmd_rx) = unbounded::<DbCommand>();
        let (init_tx, init_rx) = bounded::<HashSet<(i32, i32, i32)>>(1);

        let db_path = db_path.to_string();
        thread::spawn(move || {
            if gen_new_world {
                println!("delete old world: {db_path}");
                std::fs::remove_file(&db_path).ok();
            }

            let db = Connection::open(&db_path).expect("could not open database");
            db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .unwrap();
            Chunk::create_table(&db).expect("could not create chunk table");

            let mut generated = HashSet::new();
            {
                let mut stmt = db.prepare("SELECT x, y, z FROM chunks").unwrap();
                let mut rows = stmt.query(params![]).unwrap();
                while let Some(row) = rows.next().unwrap() {
                    let x: i32 = row.get(0).unwrap();
                    let y: i32 = row.get(1).unwrap();
                    let z: i32 = row.get(2).unwrap();
                    generated.insert((x, y, z));
                }
            }
            let _ = init_tx.send(generated);

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    DbCommand::Insert(chunk) => {
                        if let Err(e) = chunk.insert(&db) {
                            eprintln!("db failed {e}");
                        }
                    }
                    DbCommand::Find {
                        x,
                        y,
                        z,
                        response_tx,
                    } => {
                        let result = Chunk::find(&db, x, y, z).unwrap_or(None);
                        let _ = response_tx.send((x, y, z, result));
                    }
                }
            }
        });

        let generated = init_rx.recv().expect("db worker initialization failed");
        (DbWorker { cmd_tx }, generated)
    }

    fn insert(&self, chunk: Chunk) {
        let _ = self.cmd_tx.send(DbCommand::Insert(chunk));
    }

    fn find(&self, x: i32, y: i32, z: i32, response_tx: Sender<(i32, i32, i32, Option<Chunk>)>) {
        let _ = self.cmd_tx.send(DbCommand::Find {
            x,
            y,
            z,
            response_tx,
        });
    }
}

pub struct ChunkLoader {
    meshes: HashMap<(i32, i32, i32), ObjectDataID>,
    generating_chunks: HashSet<(i32, i32, i32)>,
    generated_chunks: HashSet<(i32, i32, i32)>,
    loaded_chunks: HashMap<(i32, i32, i32), Chunk>,
    loading_from_db: HashSet<(i32, i32, i32)>,
    settings: ChunkLoaderSettings,
    db_worker: DbWorker,
    db_load_tx: Sender<(i32, i32, i32, Option<Chunk>)>,
    db_load_rx: Receiver<(i32, i32, i32, Option<Chunk>)>,
    generator: ThreadPool<Chunk>,
    mesh_builder: ThreadPool<Mesh>,
}

pub struct ChunkLoaderSettings {
    pub view_distance: i32,
    pub thread_count: NonZero<usize>,
    pub db_path: String,
    pub world_height: i32,
    pub gen_new_world: bool,
}

impl ChunkLoader {
    pub fn new(settings: ChunkLoaderSettings) -> Self {
        let (db_worker, generated_chunks) =
            DbWorker::spawn(&settings.db_path, settings.gen_new_world);

        let (db_load_tx, db_load_rx) = unbounded();

        Self {
            meshes: HashMap::new(),
            generating_chunks: HashSet::new(),
            generated_chunks,
            loaded_chunks: HashMap::new(),
            loading_from_db: HashSet::new(),
            db_worker,
            db_load_tx,
            db_load_rx,
            generator: ThreadPool::new(settings.thread_count.get()),
            mesh_builder: ThreadPool::new(settings.thread_count.get()),
            settings,
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        cam_x: i32,
        cam_z: i32,
        game_object_id_count: &mut u32,
    ) {
        let view_dist = self.settings.view_distance;

        let div = |a: i32, b: i32| {
            let div = a / b;
            if div < 0 { div - 1 } else { div }
        };

        let cam_chunk_x = div(cam_x, Chunk::WIDTH as i32);
        let cam_chunk_z = div(cam_z, Chunk::WIDTH as i32);

        // Look for chunks to generate or load
        for x in (-view_dist..view_dist).map(|i| i + cam_chunk_x) {
            for z in (-view_dist..view_dist).map(|i| i + cam_chunk_z) {
                for y in 0..self.settings.world_height {
                    let pos = (x, y, z);

                    if !self.generated_chunks.contains(&pos)
                        && !self.generating_chunks.contains(&pos)
                    {
                        self.generate_chunk(x, y, z);
                    } else if self.generated_chunks.contains(&pos)
                        && !self.loaded_chunks.contains_key(&pos)
                        && !self.loading_from_db.contains(&pos)
                    {
                        self.loading_from_db.insert(pos);
                        self.db_worker.find(x, y, z, self.db_load_tx.clone());
                    }
                }
            }
        }

        // Unload chunks that are no longer needed
        self.loaded_chunks
            .retain_filter(|chunk| {
                let is_in_range = (chunk.pos[0] - cam_chunk_x).abs() <= view_dist
                    && (chunk.pos[2] - cam_chunk_z).abs() <= view_dist;

                !is_in_range
            })
            .into_iter()
            .for_each(|chunk| {
                self.db_worker.insert(chunk);
            });

        // Receive results from db loading
        let db_results: Vec<_> = self.db_load_rx.try_iter().collect();
        for (x, y, z, chunk) in db_results {
            self.loading_from_db.remove(&(x, y, z));

            if let Some(chunk) = chunk {
                self.insert_chunk(chunk);
            } else {
                eprintln!("could not find Chunk ({x},{y},{z}) in db, regenerate.");
                self.generated_chunks.remove(&(x, y, z));
                self.generate_chunk(x, y, z);
            }
        }

        // Receive generated chunks from the thread pool,
        for chunk in self.generator.results().into_iter() {
            let [x, y, z] = chunk.pos;
            self.generated_chunks.insert((x, y, z));
            self.generating_chunks.remove(&(x, y, z));
            self.insert_chunk(chunk);
        }

        // Receive generated meshes
        // and upload them to the GPU
        for mesh in self.mesh_builder.results().into_iter() {
            if mesh.vertices.is_empty() {
                continue;
            }
            let (x, y, z) = mesh.pos.into();

            let id = GameObjectID(*game_object_id_count);
            *game_object_id_count += 1;

            let instance = InstanceData::new(
                [
                    x as f32 * Chunk::WIDTH as f32,
                    y as f32 * Chunk::WIDTH as f32,
                    z as f32 * Chunk::WIDTH as f32,
                ]
                .into(),
                Quaternion::from_angle_x(Rad::from(Deg(0.0))),
            );

            println!("upload mesh");
            let object_data = renderer.instantiate_object(vec![mesh.into()], instance, id);
            self.meshes.insert((x, y, z), object_data);
        }

        let gen_tasks = self.generator.task_count();
        let mesh_tasks = self.mesh_builder.task_count();
        let db_pending = self.loading_from_db.len();
        if gen_tasks > 0 || mesh_tasks > 0 || db_pending > 0 {
            println!("world gen: {gen_tasks} | mesh build: {mesh_tasks} | db load: {db_pending}");
        }
    }

    pub fn get_chunk(&mut self, x: i32, y: i32, z: i32) -> Option<&mut Chunk> {
        self.loaded_chunks.get_mut(&(x, y, z))
    }

    pub fn get_chunks(&mut self) -> Vec<&mut Chunk> {
        self.loaded_chunks.values_mut().collect()
    }

    fn generate_chunk(&mut self, x: i32, y: i32, z: i32) {
        self.generator
            .add_task(move || world_gen::generate_chunk(x, y, z));
        self.generating_chunks.insert((x, y, z));
    }

    // add chunk to loaded chunks
    // and invoke the mesh builder
    fn insert_chunk(&mut self, chunk: Chunk) {
        let [x, y, z] = chunk.pos;
        self.loaded_chunks.insert((x, y, z), chunk);

        let mut voxels = marching_cubes::Voxels::new();
        for (nx, ny, nz) in [
            (x, y, z),
            (x + 1, y, z),
            (x, y + 1, z),
            (x, y, z + 1),
            (x + 1, y + 1, z),
            (x + 1, y, z + 1),
            (x, y + 1, z + 1),
            (x + 1, y + 1, z + 1),
        ] {
            if let Some(c) = self.loaded_chunks.get(&(nx, ny, nz)) {
                voxels.insert_chunk(c.clone());
            }
        }
        self.mesh_builder
            .add_task(move || voxels.get_chunk_mesh([x, y, z]));
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

fn is_chunk_generated(db: &Connection, x: i32, y: i32, z: i32) -> anyhow::Result<bool> {
    let mut query = db.prepare("SELECT 1 FROM chunks WHERE x = ?1 AND y = ?2 AND z = ?3")?;
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
    use std::{thread::sleep, time::Duration};

    fn setup_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        Chunk::create_table(&conn)?;
        Ok(conn)
    }

    #[test]
    fn test_create_tables() -> Result<()> {
        let conn = setup_db()?;
        let count: i32 = conn.query_row(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name IN ('chunks')",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);
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

    #[test]
    fn test_db_worker_insert_and_find() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_db_worker.db");
        let _ = std::fs::remove_file(&path);

        let (worker, initial) = DbWorker::spawn(path.to_str().unwrap(), false);
        assert!(initial.is_empty());

        let chunk = Chunk::stone_block(5, 0, 5);
        worker.insert(chunk);

        sleep(Duration::from_millis(50));

        let (tx, rx) = bounded(1);
        worker.find(5, 0, 5, tx);

        let (x, y, z, result) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!((x, y, z), (5, 0, 5));
        assert!(result.is_some());

        let _ = std::fs::remove_file(&path);
    }
}
