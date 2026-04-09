use crate::{
    engine::camera::{Camera, CameraController, Projection},
    game_object::{EndOfLife, GameObjectTrait},
};
use cgmath::{Deg, Point3, Rad};
use std::{collections::HashMap, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

pub mod camera;
pub mod marching_cubes;
mod marching_cubes_data;
pub mod physics;
pub mod renderer;
mod scene;
pub mod world_gen;

use renderer::Renderer;

pub const CHUNK_WIDTH: usize = 16;

#[derive(Clone, Copy, Debug, Default)]
pub enum Material {
    #[default]
    STONE,
    DIRT,
    GRASS,
    SNOW,
    SAND,
}

#[derive(Debug)]
pub struct Chunk {
    pub pos: [i32; 3],
    pub materials: [[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
    pub amount: [[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
}

enum Event {
    SpawnObject(Box<dyn GameObjectTrait>),
}

pub struct Engine {
    camera: Camera,
    camera_controller: CameraController,
    pub game_object_id_count: u32,
    pub renderer: Renderer,
    scene: HashMap<u32, Box<dyn GameObjectTrait>>,
}

impl Engine {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        Self {
            game_object_id_count: 0,
            scene: HashMap::new(),
            renderer: Renderer::new(&event_loop),
            camera: Camera::new(
                Point3::new(0.0, 0.0, 0.0),
                Rad::from(Deg(90.0)),
                Rad::from(Deg(0.0)),
                Projection::new(10, 10, Rad::from(Deg(90.0)), 0.1, 1000.0),
            ),
            camera_controller: CameraController::new(1.0, 2.0),
        }
    }

    fn update(&mut self) {
        let mut index: u32 = 0;
        loop {
            let object = self.scene.remove(&index);
            if let Some(mut object) = object {
                match object.update(self) {
                    EndOfLife(true) => {
                        index -= 1;
                    }
                    EndOfLife(false) => {
                        self.scene.insert(object.get_id(), object);
                        index += 1;
                    }
                }
            } else {
                break;
            }
        }
    }

    pub fn add_game_object(&mut self, object: Box<dyn GameObjectTrait>) {
        self.scene.insert(object.get_id(), object);
    }
}

impl ApplicationHandler for Engine {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.renderer.resize(window, &mut self.camera);
    }
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.camera_controller.proccess_mouse(delta.0, delta.1);

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
                self.camera_controller.update_camera(&mut self.camera);
                self.renderer.update_camera_uniform(&mut self.camera);
                self.renderer.render();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let ElementState::Pressed = event.state {
                    if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                        self.renderer.set_cursor_grab(false);
                    }
                }

                if let PhysicalKey::Code(key) = event.physical_key {
                    self.camera_controller.process_keyboard(key, event.state);
                }

                let data = self.renderer.render_data.as_mut();

                if let Some(d) = data {
                    d.window.request_redraw();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(&delta);
                if let Some(d) = self.renderer.render_data.as_ref() {
                    d.window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if button == winit::event::MouseButton::Left {
                    self.renderer.set_cursor_grab(true);
                }
            }

            _ => (),
        }
    }
}
