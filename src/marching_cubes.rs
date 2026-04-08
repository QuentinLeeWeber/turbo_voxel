use std::vec::Vec;
use std::collections::HashMap;

use crate::marching_cubes_data::*;


pub struct Voxels {
    pub chunks: Vec<Chunk>,
}

impl Voxels {
    pub fn new () -> Self {
        Self { chunks: (Vec::new()) }
    }
}

const CHUNK_WIDTH: usize = 16;    // x

pub struct Chunk {
    pub pos: [usize; 3],
    pub voxels: [[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
}

impl Chunk {
    pub fn new (x: i32, y: i32, z: i32) -> Self {
        Self { pos: [ 0,  0,  0], voxels: [[[0.0; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH] }
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

    fn add_point(&self, mesh: &mut Mesh, base: [f32; 3], offset: [f32; 3]) -> usize {
        let idx = mesh.vertexes.len();
        let p = [
            base[0] + offset[0],
            base[1] + offset[1],
            base[2] + offset[2],
        ];
        let vertex = Vertex::from_pos(p);

        mesh.vertexes.push(vertex);

        return idx;
    }

    fn add_triangle (&self, mesh: &mut Mesh, base: [f32; 3], offset: [[f32; 3]; 3]) {
        let p1 = self.add_point(mesh, base, offset[0]);
        let p2 = self.add_point(mesh, base, offset[1]);
        let p3 = self.add_point(mesh, base, offset[2]);

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

                    if case.count > 0 {
                        for triangle_idx in 0..case.count {
                            let edges = Vec3::<i8>{
                                x: case.edges[(triangle_idx*3 + 0) as usize],
                                y: case.edges[(triangle_idx*3 + 1) as usize],
                                z: case.edges[(triangle_idx*3 + 2) as usize],
                            };

                            let points = [
                                edge_idx_to_point_coord(edges.x, -1.0, 1.0),
                                edge_idx_to_point_coord(edges.y, -1.0, 1.0),
                                edge_idx_to_point_coord(edges.z, -1.0, 1.0),
                            ];

                            self.add_triangle(
                                &mut mesh,
                                [
                                    (x + self.pos[0]) as f32,
                                    (y + self.pos[1]) as f32,
                                    (z + self.pos[0]) as f32,
                                ],
                                points
                            );
                        }
                    }
                }
            }
        }




        return mesh;
    }
}

pub fn edge_idx_to_point_coord(idx: i8, v1: f32, v2: f32) -> [f32; 3] {
    let points = edge_vertex_indices[idx as usize];
    let p1 = [((points[0] >> 0) & 1) as f32, ((points[0] >> 1) & 1) as f32, ((points[0] >> 2) & 1) as f32];
    let p2 = [((points[1] >> 0) & 1) as f32, ((points[1] >> 1) & 1) as f32, ((points[1] >> 2) & 1) as f32];

    return [
        (p1[0] + p2[0])/2.0,
        (p1[1] + p2[1])/2.0,
        (p1[2] + p2[2])/2.0,
    ];
}


#[derive(Debug)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Debug)]
pub struct Vertex {
    pub pos: Vec3<f32>,
}

impl Vertex {
    pub fn from_pos (pos: [f32; 3]) -> Self {
        Self { pos: Vec3::<f32>{x: pos[0], y: pos[1], z: pos[2]} }
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
    pub hashed_points: HashMap<[usize; 4], usize>,
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


