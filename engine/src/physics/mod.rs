mod collision_detection;
pub(crate) mod hit_box;
mod octree;
mod test;

use crate::game_object::GameObjectTrait;
use cgmath::Vector3;
use hit_box::{BoundingBox, HitBox};
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
            self.stack.push((node, (*index, &hit_box)));
        }
    }

    fn remove_lower_elements(&mut self, node: u64) {
        while self.stack.last().is_some() && self.stack.last().unwrap().0 > node {
            self.stack.pop();
        }
    }
    fn collide_with_all(&mut self, index: usize, collisions: &mut HashMap<u32, ColData>) -> () {
        if index >= self.stack.len() {
            return;
        }
        let colliding_node: &(u32, &HitBox) = &self.stack[index].1;
        for i in 0..self.stack.len() {
            let cur_node: &(u32, &HitBox) = &self.stack[i].1;
            if i == index || cur_node.0 == colliding_node.0 {
                continue;
            }
            if collision_detection::check_collision(colliding_node.1, cur_node.1) {
                if cur_node.0 != MESH_GAMEOBJECT_ID {
                    let cur_collision_data: &mut ColData =
                        collisions.entry(cur_node.0).or_insert(ColData::new());

                    cur_collision_data.objects.push(colliding_node.0);
                    cur_collision_data.directions.push(
                        collision_detection::get_collision_direction(colliding_node.1, cur_node.1),
                    );
                }
                if colliding_node.0 != MESH_GAMEOBJECT_ID {
                    let colliding_node_collision_data: &mut ColData =
                        collisions.entry(colliding_node.0).or_insert(ColData::new());

                    colliding_node_collision_data.objects.push(cur_node.0);
                    colliding_node_collision_data.directions.push(
                        collision_detection::get_collision_direction(cur_node.1, colliding_node.1),
                    );
                }
            }
        }
    }
}

pub struct ColData {
    directions: Vec<Vector3<f32>>,
    objects: Vec<u32>,
}
impl ColData {
    pub fn new() -> Self {
        Self {
            directions: vec![],
            objects: vec![],
        }
    }
}

impl Into<ColInfo> for ColData {
    fn into(self) -> ColInfo {
        ColInfo {
            col_dir: self.directions.into_iter().sum(),
            objects: self.objects,
        }
    }
}

pub struct ColInfo {
    col_dir: Vector3<f32>,
    objects: Vec<u32>,
}

const MESH_GAMEOBJECT_ID: u32 = u32::MAX;

pub fn calculate_collisions(entities: &mut HashMap<u32, Box<dyn GameObjectTrait>>) -> () {
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
    for (index, entity) in &*entities {
        let hit_box = entity.get_hitbox();
        let key = octree.add_element(&hit_box);
        if key.is_none() {
            continue;
        }
        let key = key.unwrap();
        if !octree_elements.contains_key(&key) {
            octree_elements.insert(key, vec![]);
        }
        octree_elements
            .get_mut(&key)
            .unwrap()
            .push((*index, hit_box));
    }
    let mut collisions: HashMap<u32, ColData> = HashMap::new();
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
            let mut cur_obj = stack.stack.len() - 1;
            while stack.stack[cur_obj].0 == node {
                if cur_obj > 0 {
                    cur_obj -= 1;
                    stack.collide_with_all(cur_obj, &mut collisions);
                } else {
                    break;
                }
            }
        }
        prev_node = node;
    }
    for (index, _col_data) in collisions {
        let game_obj_opt = entities.get_mut(&index);
        if game_obj_opt.is_some() {
            todo!("this is for later, when we do the engine api");
            // game_obj_opt.unwrap().give_collision_info(col_data.into());
        }
    }
}
