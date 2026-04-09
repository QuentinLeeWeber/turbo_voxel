use std::{
    collections::HashMap,
    ops::{Div, Rem},
    vec::Vec,
};

use crate::engine::renderer::prelude::VertexData;

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
            amount: Box::new([[[0.0; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]),
            materials: Box::new([[[Material::default(); CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]),
        }
    }

    pub fn get_voxel(&self, voxels: &Voxels, x: usize, y: usize, z: usize) -> f32 {
        // Fast path: coordinates are within this chunk
        if x < CHUNK_WIDTH && y < CHUNK_WIDTH && z < CHUNK_WIDTH {
            return self.amount[x][y][z];
        }

        // Compute which chunk the coordinates fall into
        let chunk_x = (x / CHUNK_WIDTH) as i32 + self.pos[0];
        let chunk_y = (y / CHUNK_WIDTH) as i32 + self.pos[1];
        let chunk_z = (z / CHUNK_WIDTH) as i32 + self.pos[2];

        let idx = [chunk_x, chunk_y, chunk_z];

        if let Some(chunk) = voxels.chunks.get(&idx) {
            // Access directly without recursion to avoid stack overflow
            let lx = x % CHUNK_WIDTH;
            let ly = y % CHUNK_WIDTH;
            let lz = z % CHUNK_WIDTH;
            return chunk.amount[lx][ly][lz];
        }

        // Neighbour chunk doesn't exist – clamp to this chunk's border
        let cx = x.min(CHUNK_WIDTH - 1);
        let cy = y.min(CHUNK_WIDTH - 1);
        let cz = z.min(CHUNK_WIDTH - 1);
        self.amount[cx][cy][cz]
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

        for z in 0..CHUNK_WIDTH {
            for y in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    let idx = self.get_table_idx(voxels, x, y, z);
                    let case = &TRIANGLE_TABLE[idx as usize];

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

        smooth_mesh_laplacian(&mut mesh, 2);

        compute_normals(&mut mesh);

        mesh
    }
}

pub fn compute_normals(mesh: &mut Mesh) {
    for v in &mut mesh.vertices {
        v.normal = [0.0; 3];
    }

    for face in &mesh.faces {
        let [i0, i1, i2] = face.points;

        //vertex points
        let p0 = mesh.vertices[i0].pos;
        let p1 = mesh.vertices[i1].pos;
        let p2 = mesh.vertices[i2].pos;

        //diffs
        let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let e3 = [p2[0] - p1[0], p2[1] - p1[1], p2[2] - p1[2]];

        //cross product
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];

        let e_neg1 = [-e1[0], -e1[1], -e1[2]];
        let e_neg3 = [-e3[0], -e3[1], -e3[2]];

        let w = [
            angle_weight(e1, e2),
            angle_weight(e_neg1, e3),
            angle_weight(e_neg3, e_neg1),
        ];

        for (&idx, &w_i) in [i0, i1, i2].iter().zip(w.iter()) {
            mesh.vertices[idx].normal[0] += n[0] * w_i;
            mesh.vertices[idx].normal[1] += n[1] * w_i;
            mesh.vertices[idx].normal[2] += n[2] * w_i;
        }
    }

    for v in &mut mesh.vertices {
        let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        if len > 1e-6 {
            v.normal[0] /= len;
            v.normal[1] /= len;
            v.normal[2] /= len;
        }
    }
}

fn angle_weight(a: [f32; 3], b: [f32; 3]) -> f32 {
    let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-8);
    let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt().max(1e-8);

    ((a[0] / la) * (b[0] / lb) + (a[1] / la) * (b[1] / lb) + (a[2] / la) * (b[2] / lb))
        .clamp(-1.0, 1.0)
        .acos()
}

pub fn smooth_mesh_laplacian(mesh: &mut Mesh, iterations: usize) {
    let vertex_count = mesh.vertices.len();
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];

    for face in &mesh.faces {
        let [i0, i1, i2] = face.points;
        for (a, b) in [(i0, i1), (i1, i2), (i2, i0), (i1, i0), (i2, i1), (i0, i2)] {
            if !neighbors[a].contains(&b) {
                neighbors[a].push(b);
            }
        }
    }

    let lambda = 0.5f32;

    for _ in 0..iterations {
        let old_positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.pos).collect();

        for (i, v) in mesh.vertices.iter_mut().enumerate() {
            let nbrs = &neighbors[i];
            if nbrs.is_empty() {
                continue;
            }
            let mut avg = [0.0f32; 3];
            for &n in nbrs {
                avg[0] += old_positions[n][0];
                avg[1] += old_positions[n][1];
                avg[2] += old_positions[n][2];
            }
            let count = nbrs.len() as f32;
            v.pos[0] = v.pos[0] * (1.0 - lambda) + (avg[0] / count) * lambda;
            v.pos[1] = v.pos[1] * (1.0 - lambda) + (avg[1] / count) * lambda;
            v.pos[2] = v.pos[2] * (1.0 - lambda) + (avg[2] / count) * lambda;
        }
    }
}

pub fn edge_idx_to_point_hash(idx: i8, pos: [i32; 3], offset: [usize; 3]) -> [isize; 4] {
    let mut hash = EDGE_HASHMAP_DATA[idx as usize];
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
    let points = EDGE_VERTEX_INDICE[idx as usize];
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

    // Corrected lerp: t is the fraction along p1->p2 where the isosurface crosses zero.
    let t = val1 / (val1 - val2);

    [
        (p1[0] as f32) * (1.0 - t) + (p2[0] as f32) * t,
        (p1[1] as f32) * (1.0 - t) + (p2[1] as f32) * t,
        (p1[2] as f32) * (1.0 - t) + (p2[2] as f32) * t,
    ]
}

#[derive(Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn from_pos(pos: [f32; 3]) -> Self {
        Self {
            pos,
            normal: [0.0; 3],
        }
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

impl Into<crate::engine::renderer::prelude::MeshData> for Mesh {
    fn into(self) -> crate::engine::renderer::prelude::MeshData {
        let vertices: Vec<crate::engine::renderer::prelude::VertexData> = self
            .vertices
            .into_iter()
            .map(|v| VertexData::new(v.pos, v.normal))
            .collect();

        let indices: Vec<u32> = self
            .faces
            .into_iter()
            .map(|f| (f.points[0] as u32, f.points[1] as u32, f.points[2] as u32))
            .flat_map(|(a, b, c)| [a, b, c])
            .collect();

        crate::engine::renderer::prelude::MeshData {
            vertices,
            indices,
            material_id: 0,
        }
    }
}
