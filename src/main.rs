mod engine;

use engine::renderer::Renderer;
use engine::renderer::prelude::*;
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

fn main() {
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    let objects = vec![ObjectData {
        id: 1,
        materials: Vec::new(),
        meshes: vec![MeshData {
            id: 1,
            vertices: vec![
                VertexData::new([-0.5, -0.25, 0.0], [0.0, 0.0], [1.0, 0.0, 0.0]),
                VertexData::new([0.0, 0.5, 0.0], [0.0, 0.0], [1.0, 0.0, 0.0]),
                VertexData::new([0.25, -0.1, 0.0], [0.0, 0.0], [1.0, 0.0, 0.0]),
            ],
            indices: vec![],
            material_id: 0,
        }],
    }];
    let mut renderer = Renderer::new(&event_loop, objects); //TODO: hier alle Objekte der Szene übergeben.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut renderer).unwrap();

    println!("Hello, world!");
}
