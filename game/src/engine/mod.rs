use crate::{
    engine::{
        camera::{Camera, CameraController, Projection},
        chunk_loader::{ChunkLoader, ChunkLoaderSettings},
        renderer::Renderer,
    },
    game_object::{EndOfLife, GameObjectID, GameObjectTrait},
    hit_box::HitBox,
};
use bincode_next::{Decode, Encode};
use cgmath::{Deg, Point3, Rad};
use std::{alloc, collections::HashMap, num::NonZero, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

pub mod camera;
mod chunk_loader;
mod db_worker;
pub mod marching_cubes;
mod marching_cubes_data;
mod perlin_noise;
pub mod physics;
pub mod renderer;
mod scene;
mod thread_pool;
pub mod world_gen;

#[derive(Clone, Copy, Debug, Default, Encode, Decode, PartialEq, Eq)]
pub enum Material {
    #[default]
    Stone,
    Dirt,
    Grass,
    Snow,
    Sand,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub pos: [i32; 3],
    pub materials: Box<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>,
    pub amount: Box<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>,
}

impl Chunk {
    pub fn alloc_amount() -> Box<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]> {
        unsafe {
            let layout = alloc::Layout::new::<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>();
            let ptr = alloc::alloc_zeroed(layout)
                as *mut [[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH];
            Box::from_raw(ptr)
        }
    }

    pub fn alloc_materials() -> Box<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]> {
        unsafe {
            let layout =
                alloc::Layout::new::<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>();
            let ptr = alloc::alloc_zeroed(layout)
                as *mut [[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH];
            Box::from_raw(ptr)
        }
    }

    #[cfg(test)]
    pub fn stone_block(x: usize, y: usize, z: usize) -> Self {
        Self {
            pos: [x as i32, y as i32, z as i32],
            materials: Self::alloc_materials(),
            amount: Self::alloc_amount(),
        }
    }
}

impl Chunk {
    pub const WIDTH: usize = 128;
}

struct Transform {
    pos: [f32; 3],
    rot: [f32; 3],
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
    fn give_collision_info(&mut self, col_info: physics::ColInfo) -> ();
}

pub struct Engine {
    camera: Camera,
    camera_controller: CameraController,
    pub game_object_id_count: u32,
    pub renderer: Renderer,
    scene: HashMap<GameObjectID, Box<dyn GameObjectTrait>>,
    chunk_loader: ChunkLoader,
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
            camera_controller: CameraController::new(10.0, 1.0),
            chunk_loader: ChunkLoader::new(ChunkLoaderSettings {
                view_distance: 2,
                thread_count: std::thread::available_parallelism()
                    .unwrap_or(NonZero::new(4).unwrap()),
                db_path: "world.db".into(),
                world_height: 3,
                gen_new_world: true,
            }),
        }
    }

    fn update(&mut self) {
        let (cam_x, _cam_y, cam_z) = self.camera.position.into();
        self.chunk_loader.update(
            &mut self.renderer,
            cam_x as i32,
            cam_z as i32,
            &mut self.game_object_id_count,
        );

        let mut index: u32 = 0;
        loop {
            let object = self.scene.remove(&GameObjectID(index));
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
                if let Some(render_data) = &self.renderer.render_data {
                    render_data.window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let ElementState::Pressed = event.state
                    && let PhysicalKey::Code(KeyCode::Escape) = event.physical_key
                {
                    self.renderer.set_cursor_grab(false);
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
