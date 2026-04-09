use crate::engine::GameObjectTrait;
use std::collections::HashMap;

pub struct Scene {
    entities: HashMap<u32, Box<dyn GameObjectTrait>>,
    entity_id_counter: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            entity_id_counter: 0,
        }
    }

    pub fn add_entity(&mut self, entity: Box<dyn GameObjectTrait>) {
        self.entities.insert(self.entity_id_counter, entity);
        self.entity_id_counter += 1;
    }

    pub fn remove_entity(&mut self, entity_id: u32) {
        self.entities.remove(&entity_id);
    }
}
