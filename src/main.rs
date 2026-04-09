mod engine;

use cgmath::Deg;
use cgmath::InnerSpace;
use cgmath::Quaternion;
use cgmath::Rotation3;
use engine::Engine;
use engine::marching_cubes::Voxels;
use engine::renderer::prelude::*;
use engine::world_gen;
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

fn main() {
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    /*
    let vertices = vec![
        VertexData::new([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0]), // 0
        VertexData::new([0.5, -0.5, 0.5], [0.0, 0.0, 1.0]),  // 1
        VertexData::new([0.5, 0.5, 0.5], [0.0, 0.0, 1.0]),   // 2
        VertexData::new([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0]),  // 3
        VertexData::new([0.5, -0.5, -0.5], [0.0, 0.0, -1.0]), // 4
        VertexData::new([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0]), // 5
        VertexData::new([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0]), // 6
        VertexData::new([0.5, 0.5, -0.5], [0.0, 0.0, -1.0]), // 7
        VertexData::new([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0]),  // 8
        VertexData::new([0.5, 0.5, 0.5], [0.0, 1.0, 0.0]),   // 9
        VertexData::new([0.5, 0.5, -0.5], [0.0, 1.0, 0.0]),  // 10
        VertexData::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0]), // 11
        VertexData::new([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0]), // 12
        VertexData::new([0.5, -0.5, -0.5], [0.0, -1.0, 0.0]), // 13
        VertexData::new([0.5, -0.5, 0.5], [0.0, -1.0, 0.0]), // 14
        VertexData::new([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0]), // 15
        VertexData::new([0.5, -0.5, 0.5], [1.0, 0.0, 0.0]),  // 16
        VertexData::new([0.5, -0.5, -0.5], [1.0, 0.0, 0.0]), // 17
        VertexData::new([0.5, 0.5, -0.5], [1.0, 0.0, 0.0]),  // 18
        VertexData::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]),   // 19
        VertexData::new([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0]), // 20
        VertexData::new([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0]), // 21
        VertexData::new([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0]), // 22
        VertexData::new([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0]), // 23
    ];

    let indices = vec![
        0, 1, 2, 0, 2, 3, // Vorne
        4, 5, 6, 4, 6, 7, // Hinten
        8, 9, 10, 8, 10, 11, // Oben
        12, 13, 14, 12, 14, 15, // Unten
        16, 17, 18, 16, 18, 19, // Rechts
        20, 21, 22, 20, 22, 23, // Links
    ];

    impl Into<crate::engine::renderer::prelude::MeshData> for crate::engine::marching_cubes::Mesh {
        let vertices: Vec<VertexData> = Vec::new();
        let indices: Vec<u32> = Vec::new();
        fn into(self) -> MeshData {
            MeshData {
                id: 1,
                vertices,
                indices,
                material_id: 0,
            }
        }
    }

    let chunk = world_gen::generate_chunk(1, 1, 1);
    let voxels = Voxels::new();
    voxels.insert_chunk(chunk);
    let mesh = chunk.get_mesh(&voxels);

    let objects = vec![ObjectData {
        id: 1,
        meshes: vec![MeshData {
            id: 1,
            vertices,
            indices,
            material_id: 0,
        }],
    }]; */
    let mut engine = Engine::new(&event_loop); //TODO: hier alle Objekte der Szene übergeben.
    let axis = cgmath::Vector3::new(0.5, 0.5, 0.1).normalize();
    /*engine.renderer.add_object_instance(
        1,
        GPUInstance {
            instance_id: 1,
            instance: InstanceData::new(
                cgmath::Vector3::new(-0.1, 0.3, -0.02),
                Quaternion::from_axis_angle(axis, Deg(10.0)),
            ),
        },
    );*/
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut engine).unwrap();
}
