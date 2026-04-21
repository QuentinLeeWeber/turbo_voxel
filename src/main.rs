mod engine;
mod game_object;
mod hit_box;

use crate::{engine::marching_cubes, game_object::Transform};
use cgmath::{Deg, Quaternion, Rad, Rotation3};
use engine::{Engine, renderer::prelude::*};
use rayon::prelude::*;
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

fn main() {
    println!("main");
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
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

    let _mesh = MeshData {
        vertices,
        indices,
        material_id: 0,
    };

    let mut engine = Engine::new(&event_loop);

    println!("pre engine");

    let chunks: Vec<_> = (-1..2)
        .into_par_iter()
        .flat_map(|x| {
            (-1..2).into_par_iter().flat_map(move |y| {
                (-1..2).into_par_iter().map(move |z| {
                    let chunk = engine::world_gen::generate_chunk(x, y, z);
                    ((x, y, z), chunk) // Wir geben die Position mit zurück
                })
            })
        })
        .collect();

    let mut voxels = marching_cubes::Voxels::new();
    for (_pos, chunk) in chunks {
        voxels.insert_chunk(chunk);
    }

    let mesh = voxels.get_chunk_mesh([0, 0, 0]);

    struct ObjectData {}
    game_object::GameObjectBuilder::<ObjectData>::new(ObjectData {})
        .with_transform(Transform {
            pos: [0.0, 0.0, 0.0].into(),
            rot: Quaternion::from_angle_x(Rad::from(Deg(90.0))),
        })
        .with_mesh(mesh.into())
        .build(&mut engine);

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut engine).unwrap();
}
