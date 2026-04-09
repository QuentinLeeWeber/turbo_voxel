use std::collections::HashMap;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub mod marching_cubes;
mod marching_cubes_data;
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

struct Transform {
    pos: [f32; 3],
    rot: [f32; 3],
}

#[derive(Debug, Default)]
enum HitBox {
    #[default]
    None,
    Sphere {
        radius: f32,
    },
    Cube {
        size: f32,
    },
}

enum Event {
    SpawObject(Box<dyn GameObjectTrait>),
}

pub struct RenderInfo {
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
    pub material_id: u32,
}

trait GameObjectTrait {
    fn get_id(&self) -> u32;
    fn update(&mut self, engine: &mut Engine);
    fn get_transform(&self) -> Transform;
    fn get_hitbox(&self) -> HitBox;
    fn get_renderer_info(&self) -> RenderInfo;
}

pub struct GameObject<T> {
    pub data: T,
    id: u32,
    hitbox: HitBox,
    render_info: RenderInfo,
    control_function: Box<dyn FnMut(&mut T, &mut Engine)>,
    transform: Transform,
}

impl<T> GameObjectTrait for GameObject<T> {
    fn get_id(&self) -> u32 {
        self.id
    }
    fn update(&mut self, engine: &mut Engine) {
        (self.control_function)(&mut self.data, engine);
    }
    fn get_transform(&self) -> Transform {
        self.transform
    }
    fn get_hitbox(&self) -> HitBox {
        self.hitbox
    }
    fn get_renderer_info(&self) -> RenderInfo {
        self.render_info
    }
}

pub struct GameObjectBuilder<T> {
    data: T,
    hitbox: HitBox,
    render_info: RenderInfo,
    transform: Transform,
    control_function: Box<dyn FnMut(&mut T, &mut Engine)>,
}

impl<T> GameObjectBuilder<T> {
    pub fn new(data: T) -> Self {
        GameObjectBuilder {
            data,
            hitbox: Default::default(),
            render_info: Default::default(),
            control_function: Default::default(),
            transform: Default::default(),
        }
    }

    pub fn with_hitbox(mut self, hitbox: HitBox) -> Self {
        self.hitbox = hitbox;
        self
    }

    pub fn with_render_info(mut self, render_info: RenderInfo) -> Self {
        self.render_info = render_info;
        self
    }

    pub fn with_control<F>(mut self, control: F) -> Self
    where
        F: FnMut(&mut T, &mut Engine) + 'static,
    {
        self.control_functions.push(Box::new(control));
        self
    }

    pub fn build(self, engine: &mut Engine) {
        engine.add_game_object(Box::new(GameObject {
            id: 0,
            data: self.data,
            hitbox: self.hitbox,
            render_info: self.render_info,
            control_function: self.control_function,
            transform: self.transform,
        }));
    }
}

pub struct Engine {
    scene: HashMap<u32, Box<dyn GameObjectTrait>>,
    pub renderer: Renderer,
}

pub struct RenderInfoHash {}

pub struct Mesh {
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
}

impl Engine {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>, objects: Vec<ObjectData>) -> Self {
        Self {
            scene: HashMap::new(),
            renderer: Renderer::new(&event_loop, objects), // TODO: hier alle Objekte der Szene übergeben.
        }
    }

    fn update(&mut self) {
        for object in self.scene.values_mut() {
            object.update(self);
        }
    }

    fn add_game_object(&mut self, object: Box<dyn GameObjectTrait>) {
        self.scene.add_object(object);
    }

    fn add_mesh(&mut self, mesh: Mesh) -> RenderInfoHash {}
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
