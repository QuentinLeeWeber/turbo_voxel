use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub mod renderer;
mod scene;

use renderer::Renderer;
use scene::Scene;

use crate::engine::renderer::prelude::ObjectData;

struct Transform {
    pos: [f32; 3],
    rot: [f32; 3],
}

enum HitBox {
    None,
    Sphere { radius: f32 },
    Cube { size: f32 },
}

enum Event {
    SpawObject(Box<dyn GameObject>),
}

trait GameObject {
    fn get_id(&self) -> u32;
    fn update(&mut self);
    fn get_transform(&self) -> Transform;
    fn get_hitbox(&self) -> HitBox;
    fn notify(&mut self) -> Vec<Event>;
}

pub struct Engine {
    scene: Scene,
    pub renderer: Renderer,
}

impl Engine {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>, objects: Vec<ObjectData>) -> Self {
        Self {
            scene: Scene::new(),
            renderer: Renderer::new(&event_loop, objects), // TODO: hier alle Objekte der Szene übergeben.
        }
    }

    fn update(&mut self) {}
}

impl ApplicationHandler for Engine {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.renderer.resize(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.renderer.update_screen_size();
            }
            WindowEvent::RedrawRequested => {
                self.update();
                self.renderer.render();
            }
            _ => (),
        }
    }
}
