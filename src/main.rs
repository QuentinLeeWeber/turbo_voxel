mod engine;

use engine::Engine;
use engine::renderer::Renderer;
use engine::renderer::prelude::*;
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

fn main() {
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();

    let vertices = vec![
        // Basis: Unten-Links, Unten-Rechts, Oben-Rechts, Oben-Links
        VertexData::new([-0.5, -0.5, 0.5], [0.0, 0.0], [0.0, -1.0, 0.0]), // 0
        VertexData::new([0.5, -0.5, 0.5], [1.0, 0.0], [0.0, -1.0, 0.0]),  // 1
        VertexData::new([0.5, -0.5, -0.5], [1.0, 1.0], [0.0, -1.0, 0.0]), // 2
        VertexData::new([-0.5, -0.5, -0.5], [0.0, 1.0], [0.0, -1.0, 0.0]), // 3
        // Spitze (Oben in der Mitte)
        VertexData::new([0.0, 0.5, 0.0], [0.5, 0.5], [0.0, 1.0, 0.0]), // 4
    ];

    // 2. Die Indizes definieren (Jeweils 3 pro Dreieck)
    let indices = vec![
        // Seitenflächen (Gegen den Uhrzeigersinn für Front-Face Culling)
        0, 1, 4, // Vorne
        1, 2, 4, // Rechts
        2, 3, 4, // Hinten
        3, 0, 4, // Links
        // Basis (2 Dreiecke)
        0, 2, 1, 0, 3, 2,
    ];

    let objects = vec![ObjectData {
        id: 1,
        materials: Vec::new(),
        meshes: vec![MeshData {
            id: 1,
            vertices: vertices,
            indices: indices,
            material_id: 0,
        }],
    }];
    let mut engine = Engine::new(&event_loop, objects); //TODO: hier alle Objekte der Szene übergeben.
    engine
        .renderer
        .add_object_instance(1, InstanceData::new([0.0, 0.0, 0.0], 1.0));
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut engine).unwrap();
}
