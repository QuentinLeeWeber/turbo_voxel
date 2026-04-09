use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub mod renderer;
mod scene;
pub mod world_gen;

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
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.renderer
                .camera_controller
                .proccess_mouse(delta.0, delta.1);

            if let Some(render_data) = &self.renderer.render_data {
                render_data.window.request_redraw();
            }
        }
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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                self.renderer.camera_controller.process_keyboard(key, state);
                let data = self.renderer.render_data.as_mut();

                if let Some(d) = data {
                    d.window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.renderer.camera_controller.process_scroll(&delta);
                if let Some(d) = self.renderer.render_data.as_ref() {
                    d.window.request_redraw();
                }
            }

            _ => (),
        }
    }
}
