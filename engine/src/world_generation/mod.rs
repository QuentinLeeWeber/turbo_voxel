use crate::{
    chunk::Chunk,
    game_object::GameObjectID,
    renderer::{Renderer, prelude::InstanceData},
};
use cgmath::{Deg, Quaternion, Rad, Rotation3};
use crossbeam::channel::{Receiver, Sender, unbounded};
use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
};

mod chunk_gen;
mod db_worker;
mod marching_cubes;
mod prelude;
mod thread_pool;

use db_worker::DbWorker;
use prelude::*;
use thread_pool::ThreadPool;

pub struct ChunkLoader {
    meshes: HashMap<(i32, i32, i32), (GameObjectID, i32)>,
    generating_chunks: HashSet<(i32, i32, i32)>,
    generated_chunks: HashSet<(i32, i32, i32)>,
    loaded_chunks: HashMap<(i32, i32, i32), Chunk>,
    loading_from_db: HashSet<(i32, i32, i32)>,
    settings: ChunkLoaderSettings,
    db_worker: DbWorker,
    db_load_tx: Sender<(i32, i32, i32, Option<Chunk>)>,
    db_load_rx: Receiver<(i32, i32, i32, Option<Chunk>)>,
    generator: ThreadPool<Chunk>,
    mesh_builder: ThreadPool<(marching_cubes::Mesh, i32)>,
    mesh_count: i32,
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
            generator: ThreadPool::new(
                settings.thread_count.get(),
                Some("chunk_generator".to_string()),
            ),
            mesh_builder: ThreadPool::new(
                settings.thread_count.get(),
                Some("mesh_builder".to_string()),
            ),
            settings,
            mesh_count: 0,
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
                let [x, y, z] = chunk.pos;
                if let Some((game_object_id, _)) = self.meshes.remove(&(x, y, z)) {
                    renderer.remove_game_object(game_object_id);
                }
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
        for (mesh, mesh_id) in self.mesh_builder.results().into_iter() {
            if mesh.vertices.is_empty() {
                continue;
            }
            let (x, y, z) = mesh.pos.into();

            let game_object_id = GameObjectID(*game_object_id_count);
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

            let _object_data =
                renderer.instantiate_object(vec![mesh.into()], instance, game_object_id);
            if let Some((game_object_id, count)) = self.meshes.get(&(x, y, z)) {
                if mesh_id > *count {
                    renderer.remove_game_object(*game_object_id);
                    self.meshes.insert((x, y, z), (*game_object_id, mesh_id));
                }
            } else {
                self.meshes.insert((x, y, z), (game_object_id, mesh_id));
            }
        }

        let gen_tasks = self.generator.task_count();
        let mesh_tasks = self.mesh_builder.task_count();
        let db_pending = self.loading_from_db.len();
        if gen_tasks > 0 || mesh_tasks > 0 || db_pending > 0 {
            println!("world gen: {gen_tasks} | mesh build: {mesh_tasks} | db load: {db_pending}");
        }
    }

    fn generate_chunk(&mut self, x: i32, y: i32, z: i32) {
        self.generator
            .add_task(move || chunk_gen::generate(x, y, z));
        self.generating_chunks.insert((x, y, z));
    }

    // add chunk to loaded chunks
    // and invoke the mesh builder
    fn insert_chunk(&mut self, chunk: Chunk) {
        let [x, y, z] = chunk.pos;
        self.loaded_chunks.insert((x, y, z), chunk);

        for dx in 0..=1i32 {
            for dy in 0..=1i32 {
                for dz in 0..=1i32 {
                    self.invoke_mesh_generation(x + dx, y + dy, z + dz);
                }
            }
        }
    }

    // Invoke mesh generation for a chunk,
    // TODO: only invoke if no neighbor chunks are still generating or loading from the database.
    fn invoke_mesh_generation(&mut self, x: i32, y: i32, z: i32) {
        if self.loaded_chunks.get(&(x, y, z)).is_none() {
            return;
        }

        let mut voxels = marching_cubes::Voxels::new();
        for dx in 0..=1i32 {
            for dy in 0..=1i32 {
                for dz in 0..=1i32 {
                    let pos = (x + dx, y + dy, z + dz);
                    if let Some(c) = self.loaded_chunks.get(&pos) {
                        voxels.insert_chunk(c.clone());
                    }
                }
            }
        }

        let mesh_count = self.mesh_count;
        self.mesh_count += 1;
        self.mesh_builder
            .add_task(move || (voxels.get_chunk_mesh([x, y, z]), mesh_count));
    }
}
