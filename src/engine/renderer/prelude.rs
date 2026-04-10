use cgmath::Matrix4;

use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

use crate::engine::renderer::{GameObjectID, ObjectDataID};

/*
 * a low level object that can be loaded from a file
 */

#[derive(Debug, Clone)]
pub struct ObjectData {
    pub id: ObjectDataID,
    pub meshes: Vec<u32>, //list of mesh ids
}

/*
 * a low level loaded Mesh
 */
#[derive(Debug, Clone)]
pub struct MeshData {
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
    pub material_id: u32,
}
impl MeshData {
    pub fn new(vertices: Vec<VertexData>, indices: Vec<u32>, material_id: u32) -> MeshData {
        MeshData {
            vertices,
            indices,
            material_id,
        }
    }
}
/*
 * a low level Vertex for rendering
 */
#[derive(BufferContents, Vertex, Debug, Clone, Copy)]
#[repr(C)]
pub struct VertexData {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub color: [f32; 3],
}
impl VertexData {
    pub fn new(pos: [f32; 3], normal: [f32; 3]) -> VertexData {
        VertexData {
            position: pos,
            normal,
            color: [1.0, 1.0, 1.0],
        }
    }
    pub fn new_with_color(pos: [f32; 3], normal: [f32; 3], color: [f32; 3]) -> VertexData {
        VertexData {
            position: pos,
            normal,
            color,
        }
    }
}
#[derive(Copy, Clone)]
pub struct GPUInstance {
    pub instance_id: GameObjectID, //die globale ID
    pub instance: InstanceData,
}

#[derive(BufferContents, Vertex, Copy, Clone)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32B32A32_SFLOAT)]
    pub model_mat: [[f32; 4]; 4],
}

impl InstanceData {
    pub fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>) -> InstanceData {
        let model_mat = Matrix4::from_translation(position) * Matrix4::from(rotation);
        InstanceData {
            model_mat: model_mat.into(),
        }
    }
}
