use cgmath::{Matrix3, Matrix4};
use std::sync::Arc;
use vulkano::buffer::Subbuffer;

use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::device::Device;
use vulkano::image::Image;
use vulkano::{
    buffer::{Buffer, BufferContents},
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::DescriptorSet,
    device::Queue,
    image::{sampler::Sampler, view::ImageView},
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::vertex_input::Vertex,
};

/*
 * Meshes and Materials of a Object type
 */
pub trait RenderResource {}

pub struct LoadedTexture {
    pub image: Arc<Image>,
    pub view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
    pub descriptor_set: Arc<DescriptorSet>,
}
/*
 * Instance of a Object to be rendererd
 */
pub trait Texture {
    fn load_textures(
        &self,
        allocator: std::sync::Arc<vulkano::memory::allocator::StandardMemoryAllocator>,
        command_buffer_allocator: Arc<
            vulkano::command_buffer::allocator::StandardCommandBufferAllocator,
        >,
        queue: std::sync::Arc<vulkano::device::Queue>,
        layout: Arc<DescriptorSetLayout>,
        device: Arc<Device>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Vec<LoadedTexture>;
}
pub trait Mesh {
    fn load_mesh();
}

/*
 * a low level object that can be loaded from a file
 */
//TODO: implement loading into and unloading from renderer
//TODO: implement rendering in renderer
#[derive(Debug, Clone)]
pub struct ObjectData {
    pub id: u32,
    pub materials: Vec<MaterialData>,
    pub meshes: Vec<MeshData>,
}
#[derive(Debug, Clone)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
impl ImageData {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> ImageData {
        ImageData {
            width,
            height,
            data,
        }
    }
}

/*
 * A low level loaded Material
 */
#[derive(Debug, Clone)]
pub struct MaterialData {
    pub name: String,
    pub diffuse_texture: ImageData,
}

/*
 * a low level loaded Mesh
 */
#[derive(Debug, Clone)]
pub struct MeshData {
    pub id: u32,
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
    pub material_id: u32,
}
impl MeshData {
    pub fn new(
        id: u32,
        vertices: Vec<VertexData>,
        indices: Vec<u32>,
        material_id: u32,
    ) -> MeshData {
        return MeshData {
            id,
            vertices,
            indices,
            material_id,
        };
    }
}
/*
 * a low level Vertex for rendering
 */
#[derive(BufferContents, Vertex, Debug, Clone, Copy)]
#[repr(C)]
pub struct VertexData {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub tex_coords: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
}
impl VertexData {
    pub fn new(pos: [f32; 3], tex_coords: [f32; 2], normal: [f32; 3]) -> VertexData {
        return VertexData {
            position: pos,
            tex_coords: tex_coords,
            normal: normal,
        };
    }
}

#[derive(BufferContents, Vertex, Copy, Clone)]
#[repr(C)]
pub struct InstanceData {
    // A 2D array maps perfectly to a mat4 in GLSL and automatically
    // consumes locations 1, 2, 3, and 4.
    #[format(R32G32B32A32_SFLOAT)]
    pub model_mat: [[f32; 4]; 4],
}

impl InstanceData {
    pub fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>) -> InstanceData {
        let model_mat = Matrix4::from_translation(position) * Matrix4::from(rotation);
        return InstanceData {
            model_mat: model_mat.into(),
        };
    }
}
