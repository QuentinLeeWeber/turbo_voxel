mod engine;

use engine::Engine;
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

fn main() {
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    let mut engine = Engine::new(&event_loop);
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut engine).unwrap();
}
