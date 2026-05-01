mod octree;
mod test;
use crate::{
    engine::GameObjectTrait,
    hit_box::{BoundingBox, HitBox},
};
use octree::*;
use std::collections::HashMap;

pub struct CoordinateBorders {
    pub lower: f32,
    pub upper: f32,
}

impl CoordinateBorders {
    pub fn get_middle(&self) -> f32 {
        (self.upper + self.lower) / 2.
    }
    pub fn new(a: f32, b: f32) -> Self {
        if a < b {
            Self { lower: a, upper: b }
        } else {
            Self { lower: b, upper: a }
        }
    }
    pub fn from_parent(parent: &CoordinateBorders, lower_half: bool) -> Self {
        if lower_half {
            Self {
                lower: parent.lower,
                upper: parent.get_middle(),
            }
        } else {
            Self {
                lower: parent.get_middle(),
                upper: parent.upper,
            }
        }
    }
    pub fn is_within(&self, other: &CoordinateBorders) -> bool {
        self.lower >= other.lower && self.upper <= other.upper
    }
}

pub struct CollisionStack<'a> {
    stack: Vec<(u64, (u32, &'a HitBox))>,
}

impl<'a> CollisionStack<'a> {
    fn new() -> Self {
        Self { stack: vec![] }
    }
    fn add_node_elements(
        &mut self,
        octree_elements: &'a HashMap<u64, Vec<(u32, HitBox)>>,
        node: u64,
    ) {
        if !octree_elements.contains_key(&node) {
            return;
        }
        for (index, hit_box) in octree_elements.get(&node).unwrap() {
            self.stack.push((node, (*index, hit_box)));
        }
    }

    fn remove_lower_elements(&mut self, node: u64) {
        while self.stack.last().is_some() && self.stack.last().unwrap().0 > node {
            self.stack.pop();
        }
    }
}

const MESH_GAMEOBJECT_ID: u32 = u32::MAX;

pub fn calculate_collisions(entities: &mut HashMap<u32, Box<dyn GameObjectTrait>>) {
    let mut octree_elements: HashMap<u64, Vec<(u32, HitBox)>> = HashMap::new();
    let mut octree = Octree::new(
        CoordinateBorders {
            lower: -5000.,
            upper: 5000.,
        },
        CoordinateBorders {
            lower: -5000.,
            upper: 5000.,
        },
        CoordinateBorders {
            lower: -5000.,
            upper: 5000.,
        },
    );
    for (index, entity) in entities {
        let hit_box = entity.get_hitbox();
        let key = octree.add_element(&hit_box);
        if key.is_none() {
            continue;
        }
        let key = key.unwrap();
        octree_elements.entry(key).or_default();
        octree_elements
            .get_mut(&key)
            .unwrap()
            .push((*index, hit_box));
    }
    let _collisions: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut stack: CollisionStack = CollisionStack::new();
    stack.add_node_elements(&octree_elements, 0);
    let mut prev_node: u64 = 0;
    for node in &octree {
        if node < prev_node {
            //step up the octree
            stack.remove_lower_elements(node);
        } else {
            //step down the octree
            stack.add_node_elements(&octree_elements, node);
        }
        prev_node = node;
    }
}

fn check_collision(_obj_1: &HitBox, _obj_2: &HitBox) -> bool {
    true
}
