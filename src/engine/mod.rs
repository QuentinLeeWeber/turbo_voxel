use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

mod marching_cubes;
mod marching_cubes_data;
mod physics;
pub mod renderer;
mod scene;
pub mod world_gen;

use renderer::Renderer;
use scene::Scene;

use crate::engine::renderer::prelude::ObjectData;

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

use physics::CoordinateBorders;

struct Transform {
    pos: [f32; 3],
    rot: [f32; 3],
}

pub struct BoundingBox {
    x: CoordinateBorders,
    y: CoordinateBorders,
    z: CoordinateBorders,
}

enum HitBox {
    None,
    Sphere {
        transform: Transform,
        radius: f32,
    },
    Cube {
        transform: Transform,
        size: f32,
    },
    Triangle {
        point1: [f32; 3],
        point2: [f32; 3],
        point3: [f32; 3],
    },
}
impl HitBox {
    pub fn get_bounding_box(&self) -> Option<BoundingBox> {
        match self {
            HitBox::None => None,
            HitBox::Sphere { transform, radius } => Some(BoundingBox {
                x: CoordinateBorders::new(transform.pos[0] - radius, transform.pos[0] + radius),
                y: CoordinateBorders::new(transform.pos[1] - radius, transform.pos[1] + radius),
                z: CoordinateBorders::new(transform.pos[2] - radius, transform.pos[2] + radius),
            }),
            HitBox::Cube { transform, size } => {
                let max_dist = (3.0f32).sqrt() / 2. * size;
                Some(BoundingBox {
                    x: CoordinateBorders::new(
                        transform.pos[0] - max_dist,
                        transform.pos[0] + max_dist,
                    ),
                    y: CoordinateBorders::new(
                        transform.pos[1] - max_dist,
                        transform.pos[1] + max_dist,
                    ),
                    z: CoordinateBorders::new(
                        transform.pos[2] - max_dist,
                        transform.pos[2] + max_dist,
                    ),
                })
            }
            HitBox::Triangle {
                point1,
                point2,
                point3,
            } => {
                let min_x = point1[0].min(point2[0].min(point3[0]));
                let max_x = point1[0].max(point2[0].max(point3[0]));
                let min_y = point1[1].min(point2[1].min(point3[1]));
                let max_y = point1[1].max(point2[1].max(point3[1]));
                let min_z = point1[2].min(point2[2].min(point3[2]));
                let max_z = point1[2].max(point2[2].max(point3[2]));
                Some(BoundingBox {
                    x: CoordinateBorders::new(min_x, max_x),
                    y: CoordinateBorders::new(min_y, max_y),
                    z: CoordinateBorders::new(min_z, max_z),
                })
            }
        }
    }
}

enum Event {
    SpawnObject(Box<dyn GameObject>),
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
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
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
