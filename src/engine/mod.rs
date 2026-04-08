pub mod renderer;
mod scene;

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

struct Engine {
    scene: scene::Scene,
}
