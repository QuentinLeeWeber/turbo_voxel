use std::vec::Vec;
use std::collections::HashMap;

use crate::marching_cubes_data::*;


pub struct Voxels {
    pub chunks: HashMap<[i32; 3],Chunk>,
}

impl Voxels {
    pub fn new () -> Self {
        Self { chunks: HashMap::new() }
    }
}

const CHUNK_WIDTH: usize = 16;    // x

pub struct Chunk {
    pub pos: [i32; 3],
    pub voxels: [[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
}

impl Chunk {
    pub fn new (x: i32, y: i32, z: i32) -> Self {
        Self { pos: [ x,  y,  z], voxels: [[[0.0; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH] }
    }

    pub fn get_voxel (&self, x: usize, y: usize, z: usize) -> f32 {
        self.voxels[x][y][z]
    }
    
    pub fn set_voxel (&mut self, x: usize, y: usize, z: usize, val: f32) {
        self.voxels[x][y][z] = val;
    }

    fn get_table_idx (&self, x: usize, y: usize, z: usize) -> u8 {
        let mut idx: u8 = 0;
        for dz in 0..2 {
            for dy in 0..2 {
                for dx in 0..2 {
                    idx |= ((self.get_voxel(x+dx, y+dy, z+dz) > 0.0) as u8) << (dx + 2*dy + 4*dz);
                }
            }
        }

        return idx;
    }

    fn add_point(&self, mesh: &mut Mesh, base: [f32; 3], offset: [f32; 3], hash: [isize; 4]) -> usize {
        if mesh.hashed_points.contains_key(&hash) {
            return mesh.hashed_points[&hash];
        }


        let idx = mesh.vertexes.len();
        let p = [
            base[0] + offset[0],
            base[1] + offset[1],
            base[2] + offset[2],
        ];
        let vertex = Vertex::from_pos(p);

        mesh.hashed_points.insert(hash, idx);
        mesh.vertexes.push(vertex);

        return idx;
    }

    fn add_triangle (&self, mesh: &mut Mesh, base: [f32; 3], offset: [[f32; 3]; 3], hashes: [[isize; 4]; 3]) {
        let p1 = self.add_point(mesh, base, offset[0], hashes[0]);
        let p2 = self.add_point(mesh, base, offset[1], hashes[1]);
        let p3 = self.add_point(mesh, base, offset[2], hashes[2]);

        let triangle = Triangle::from_idxs([p1, p2, p3]);

        mesh.triangles.push(triangle);
    }


    pub fn to_mesh (&self) -> Mesh {
        let mut mesh = Mesh::new();
        
        for z in 0..15 {
            for y in 0..15 {
                for x in 0..15 {
                    let idx = self.get_table_idx(x, y, z);
                    let case = &triangle_table[idx as usize];

                    if case.count == 0 {
                        continue;
                    }
                    for triangle_idx in 0..case.count {
                        let edges = [
                            case.edges[(triangle_idx*3 + 0) as usize],
                            case.edges[(triangle_idx*3 + 1) as usize],
                            case.edges[(triangle_idx*3 + 2) as usize],
                        ];

                        let points = [
                            edge_idx_to_point_coord(self, [x, y, z], edges[0], -1.0, 1.0),
                            edge_idx_to_point_coord(self, [x, y, z], edges[1], -1.0, 1.0),
                            edge_idx_to_point_coord(self, [x, y, z], edges[2], -1.0, 1.0),
                        ];

                        let hashes = [
                            edge_idx_to_point_hash(edges[0], self.pos, [x, y, z]),
                            edge_idx_to_point_hash(edges[1], self.pos, [x, y, z]),
                            edge_idx_to_point_hash(edges[2], self.pos, [x, y, z]),
                        ];

                        self.add_triangle(
                            &mut mesh,
                            [
                                ((x as i32) + self.pos[0]) as f32,
                                ((y as i32) + self.pos[1]) as f32,
                                ((z as i32) + self.pos[2]) as f32,
                            ],
                            points,
                            hashes
                        );
                    }
                }
            }
        }




        return mesh;
    }
}

pub fn edge_idx_to_point_hash(idx: i8, pos: [i32; 3], offset: [usize; 3]) -> [isize; 4] {
    let mut hash = edge_hashmap_data[idx as usize];
    hash[0] += (pos[0] as isize) * (CHUNK_WIDTH as isize) + (offset[0] as isize);
    hash[1] += (pos[1] as isize) * (CHUNK_WIDTH as isize) + (offset[1] as isize);
    hash[2] += (pos[2] as isize) * (CHUNK_WIDTH as isize) + (offset[2] as isize);

    return hash;
}

pub fn edge_idx_to_point_coord(chunk: &Chunk, pos: [usize; 3], idx: i8, v1: f32, v2: f32) -> [f32; 3] {
    let points = edge_vertex_indices[idx as usize];
    let p1 = [(points[0] >> 0) & 1, (points[0] >> 1) & 1, (points[0] >> 2) & 1];
    let p2 = [(points[1] >> 0) & 1, (points[1] >> 1) & 1, (points[1] >> 2) & 1];

    let val1 = chunk.get_voxel(
        p1[0] as usize + pos[0],
        p1[1] as usize + pos[1],
        p1[2] as usize + pos[2]
    );
    let val2 = chunk.get_voxel(
        p2[0] as usize + pos[0],
        p2[1] as usize + pos[1],
        p2[2] as usize + pos[2]
    );

    let lerp = -val2 / (val1 - val2);

    return [
        (p1[0] as f32)*lerp + (p2[0] as f32)*(1.0 - lerp),
        (p1[1] as f32)*lerp + (p2[1] as f32)*(1.0 - lerp),
        (p1[2] as f32)*lerp + (p2[2] as f32)*(1.0 - lerp),
    ];
}


#[derive(Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
}

impl Vertex {
    pub fn from_pos (pos: [f32; 3]) -> Self {
        Self { pos: pos }
    }
}

#[derive(Debug)]
pub struct Triangle {
    pub points: [usize; 3],
}

impl Triangle {
    pub fn from_idxs (idxs: [usize; 3]) -> Self {
        Self { points: idxs }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub vertexes: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
    pub hashed_points: HashMap<[isize; 4], usize>,
}

impl Mesh {
    pub fn new () -> Self {
        Self {
            vertexes: Vec::new(),
            triangles: Vec::new(),
            hashed_points: HashMap::new(),
        }
    }
}


