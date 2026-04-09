use std::{
    collections::HashMap,
    ops::{Div, Rem},
    vec::Vec,
};

use super::{CHUNK_WIDTH, Chunk, Material, marching_cubes_data::*};

pub struct Voxels {
    pub chunks: HashMap<[i32; 3], Chunk>,
}

impl Voxels {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn insert_chunk(&mut self, chunk: Chunk) -> &Chunk {
        let pos = chunk.pos;

        assert!(!self.chunks.contains_key(&pos));

        self.chunks.insert(pos, chunk);

        self.get_chunk(pos)
    }

    pub fn get_chunk(&self, pos: [i32; 3]) -> &Chunk {
        assert!(self.chunks.contains_key(&pos));
        &self.chunks[&pos]
    }

    pub fn get_mut_chunk(&mut self, pos: [i32; 3]) -> &mut Chunk {
        assert!(self.chunks.contains_key(&pos));
        self.chunks.get_mut(&pos).unwrap()
    }

    pub fn del_chunk(&mut self, pos: [i32; 3]) -> Option<Chunk> {
        assert!(self.chunks.contains_key(&pos));
        self.chunks.remove(&pos)
    }

    pub fn new_chunk(&mut self, pos: [i32; 3]) -> &mut Chunk {
        let chunk = Chunk::new(pos);
        self.insert_chunk(chunk);
        self.get_mut_chunk(pos)
    }

    pub fn get_chunk_mesh(&self, pos: [i32; 3]) -> Mesh {
        let chunk = &self.chunks[&pos];

        chunk.get_mesh(self)
    }
}

impl Chunk {
    pub fn new(pos: [i32; 3]) -> Self {
        Self {
            pos,
            amount: [[[0.0; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
            materials: [[[Material::default(); CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
        }
    }

    pub fn get_voxel(&self, voxels: &Voxels, x: usize, y: usize, z: usize) -> f32 {
        if (x < CHUNK_WIDTH) && (y < CHUNK_WIDTH) && (z < CHUNK_WIDTH) {
            return self.amount[x][y][z];
        }
        let chunk_x = (x.div(CHUNK_WIDTH) as i32) + self.pos[0];
        let chunk_y = (y.div(CHUNK_WIDTH) as i32) + self.pos[1];
        let chunk_z = (z.div(CHUNK_WIDTH) as i32) + self.pos[2];

        let idx = [chunk_x, chunk_y, chunk_z];
        if voxels.chunks.contains_key(&idx) {
            let chunk = &voxels.chunks[&idx];
            let new_x = x.rem(CHUNK_WIDTH);
            let new_y = y.rem(CHUNK_WIDTH);
            let new_z = z.rem(CHUNK_WIDTH);

            return chunk.get_voxel(voxels, new_x, new_y, new_z);
        }

        let new_x = x.min(CHUNK_WIDTH - 1);
        let new_y = y.min(CHUNK_WIDTH - 1);
        let new_z = z.min(CHUNK_WIDTH - 1);

        self.amount[new_x][new_y][new_z]
    }

    pub fn set_voxel(&mut self, x: usize, y: usize, z: usize, val: f32) {
        self.amount[x][y][z] = val;
    }

    fn get_table_idx(&self, voxels: &Voxels, x: usize, y: usize, z: usize) -> u8 {
        let mut idx: u8 = 0;
        for dz in 0..2 {
            for dy in 0..2 {
                for dx in 0..2 {
                    idx |= ((self.get_voxel(voxels, x + dx, y + dy, z + dz) > 0.0) as u8)
                        << (dx + 2 * dy + 4 * dz);
                }
            }
        }

        idx
    }

    fn add_point(
        &self,
        mesh: &mut Mesh,
        base: [f32; 3],
        offset: [f32; 3],
        hash: [isize; 4],
    ) -> usize {
        if mesh.hashed_points.contains_key(&hash) {
            return mesh.hashed_points[&hash];
        }

        let idx = mesh.vertices.len();
        let p = [
            base[0] + offset[0],
            base[1] + offset[1],
            base[2] + offset[2],
        ];
        let vertex = Vertex::from_pos(p);

        mesh.hashed_points.insert(hash, idx);
        mesh.vertices.push(vertex);

        idx
    }

    fn add_face(
        &self,
        mesh: &mut Mesh,
        base: [f32; 3],
        offset: [[f32; 3]; 3],
        hashes: [[isize; 4]; 3],
    ) {
        let p1 = self.add_point(mesh, base, offset[0], hashes[0]);
        let p2 = self.add_point(mesh, base, offset[1], hashes[1]);
        let p3 = self.add_point(mesh, base, offset[2], hashes[2]);

        let face = Face::from_idxs([p1, p2, p3]);

        mesh.faces.push(face);
    }

    pub fn get_mesh(&self, voxels: &Voxels) -> Mesh {
        let mut mesh = Mesh::new();

        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    let idx = self.get_table_idx(voxels, x, y, z);
                    let case = &triangle_table[idx as usize];

                    if case.count == 0 {
                        continue;
                    }
                    for face_idx in 0..case.count {
                        let edges = [
                            case.edges[(face_idx * 3) as usize],
                            case.edges[(face_idx * 3 + 1) as usize],
                            case.edges[(face_idx * 3 + 2) as usize],
                        ];

                        let points = [
                            edge_idx_to_point_coord(self, voxels, [x, y, z], edges[0]),
                            edge_idx_to_point_coord(self, voxels, [x, y, z], edges[1]),
                            edge_idx_to_point_coord(self, voxels, [x, y, z], edges[2]),
                        ];

                        let hashes = [
                            edge_idx_to_point_hash(edges[0], self.pos, [x, y, z]),
                            edge_idx_to_point_hash(edges[1], self.pos, [x, y, z]),
                            edge_idx_to_point_hash(edges[2], self.pos, [x, y, z]),
                        ];

                        self.add_face(
                            &mut mesh,
                            [
                                ((x as i32) + self.pos[0] * (CHUNK_WIDTH as i32)) as f32,
                                ((y as i32) + self.pos[1] * (CHUNK_WIDTH as i32)) as f32,
                                ((z as i32) + self.pos[2] * (CHUNK_WIDTH as i32)) as f32,
                            ],
                            points,
                            hashes,
                        );
                    }
                }
            }
        }
        mesh
    }
}

pub fn edge_idx_to_point_hash(idx: i8, pos: [i32; 3], offset: [usize; 3]) -> [isize; 4] {
    let mut hash = edge_hashmap_data[idx as usize];
    hash[0] += (pos[0] as isize) * (CHUNK_WIDTH as isize) + (offset[0] as isize);
    hash[1] += (pos[1] as isize) * (CHUNK_WIDTH as isize) + (offset[1] as isize);
    hash[2] += (pos[2] as isize) * (CHUNK_WIDTH as isize) + (offset[2] as isize);

    hash
}

pub fn edge_idx_to_point_coord(
    chunk: &Chunk,
    voxels: &Voxels,
    pos: [usize; 3],
    idx: i8,
) -> [f32; 3] {
    let points = edge_vertex_indices[idx as usize];
    let p1 = [points[0] & 1, (points[0] >> 1) & 1, (points[0] >> 2) & 1];
    let p2 = [points[1] & 1, (points[1] >> 1) & 1, (points[1] >> 2) & 1];

    let val1 = chunk.get_voxel(
        voxels,
        p1[0] as usize + pos[0],
        p1[1] as usize + pos[1],
        p1[2] as usize + pos[2],
    );
    let val2 = chunk.get_voxel(
        voxels,
        p2[0] as usize + pos[0],
        p2[1] as usize + pos[1],
        p2[2] as usize + pos[2],
    );

    let lerp = -val2 / (val1 - val2);

    [
        (p1[0] as f32) * lerp + (p2[0] as f32) * (1.0 - lerp),
        (p1[1] as f32) * lerp + (p2[1] as f32) * (1.0 - lerp),
        (p1[2] as f32) * lerp + (p2[2] as f32) * (1.0 - lerp),
    ]
}

#[derive(Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
}

impl Vertex {
    pub fn from_pos(pos: [f32; 3]) -> Self {
        Self { pos }
    }
}

#[derive(Debug)]
pub struct Face {
    pub points: [usize; 3],
}

impl Face {
    pub fn from_idxs(idxs: [usize; 3]) -> Self {
        Self { points: idxs }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub hashed_points: HashMap<[isize; 4], usize>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            hashed_points: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.faces.clear();
        self.hashed_points.clear();
    }
}
