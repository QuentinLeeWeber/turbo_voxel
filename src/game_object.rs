use crate::engine::{
    Engine,
    renderer::{ObjectDataID, prelude::*},
};
use crate::hit_box::HitBox;
use cgmath::{Deg, Quaternion, Rad, Rotation3, Vector3, Zero};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub pos: Vector3<f32>,
    pub rot: Quaternion<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vector3::zero(),
            rot: Quaternion::zero(),
        }
    }
}

pub trait GameObjectTrait {
    fn get_id(&self) -> u32;
    fn update(&mut self, engine: &mut Engine) -> EndOfLife;
    fn get_transform(&self) -> Transform;
    fn get_hitbox(&self) -> HitBox;
    fn get_object_data(&self) -> ObjectDataID; //returns basic data like meshes needed for rendering
}

pub struct GameObject<T> {
    pub data: T,
    id: u32,
    hitbox: HitBox,
    control_function: Option<Box<dyn FnMut(&mut T, &mut Engine) -> EndOfLife>>,
    transform: Transform,
    object_data: ObjectDataID,
}

impl<T> GameObjectTrait for GameObject<T> {
    fn get_id(&self) -> u32 {
        self.id
    }
    fn update(&mut self, engine: &mut Engine) -> EndOfLife {
        if let Some(control) = &mut self.control_function {
            control(&mut self.data, engine)
        } else {
            EndOfLife(false)
        }
    }
    fn get_transform(&self) -> Transform {
        self.transform
    }
    fn get_hitbox(&self) -> HitBox {
        self.hitbox
    }
    fn get_object_data(&self) -> ObjectDataID {
        self.object_data
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EndOfLife(pub bool);

pub struct GameObjectBuilder<T> {
    data: T,
    hitbox: HitBox,
    transform: Transform,
    control_function: Option<Box<dyn FnMut(&mut T, &mut Engine) -> EndOfLife>>,
    object_data: Vec<MeshData>,
}

impl<T: 'static> GameObjectBuilder<T> {
    pub fn new(data: T) -> Self {
        GameObjectBuilder {
            data,
            hitbox: Default::default(),
            control_function: None,
            transform: Default::default(),
            object_data: Vec::new(),
        }
    }

    pub fn with_hitbox(mut self, hitbox: HitBox) -> Self {
        self.hitbox = hitbox;
        self
    }

    pub fn with_control<F>(mut self, control: F) -> Self
    where
        F: FnMut(&mut T, &mut Engine) -> EndOfLife + 'static,
    {
        self.control_function = Some(Box::new(control));
        self
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_mesh(mut self, mesh: MeshData) -> Self {
        self.object_data.push(mesh);
        self
    }

    pub fn with_meshes(mut self, meshes: Vec<MeshData>) -> Self {
        self.object_data.extend(meshes);
        self
    }

    pub fn build(mut self, engine: &mut Engine) {
        let id = engine.game_object_id_count;
        engine.game_object_id_count += 1;

        let object_data = engine.renderer.instantiate_object(
            self.object_data,
            InstanceData::new(self.transform.pos, self.transform.rot),
        );

        engine.add_game_object(Box::new(GameObject {
            id,
            data: self.data,
            hitbox: self.hitbox,
            control_function: self.control_function,
            transform: self.transform,
            object_data,
        }));
    }
}
