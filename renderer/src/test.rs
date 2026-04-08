#[cfg(test)]
mod renderer_tests {
    use std::sync::{Mutex, OnceLock};

    use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11};

    use crate::renderer::Renderer;

    #[test]
    fn test_start_renderer() {
        let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
        let mut renderer = Renderer::new(&event_loop);
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        event_loop.run_app(&mut renderer).unwrap();
        assert!(true);
    }
}
