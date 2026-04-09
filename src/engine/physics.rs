use crate::engine::BoundingBox;
use crate::engine::{GameObject, HitBox};
use cgmath::SquareMatrix;
use cgmath::Vector3;
use std::char::from_u32;
use std::collections::HashMap;

pub struct CoordinateBorders {
    lower: f32,
    upper: f32,
}

impl CoordinateBorders {
    pub fn get_middle(&self) -> f32 {
        return (self.upper + self.lower) / 2.;
    }
    pub fn new(lower: f32, upper: f32) -> Self {
        Self {
            lower: lower,
            upper: upper,
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
        return self.lower >= other.lower && self.upper <= other.upper;
    }
}

struct Octree {
    nodes: HashMap<u64, OctreeNode>,
}

impl<'a> IntoIterator for &'a Octree {
    type Item = u64;
    type IntoIter = OctreeIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        OctreeIterator::new(self, 0)
    }
}

impl Octree {
    pub fn add_element(&mut self, obj: &crate::engine::HitBox) -> Option<u64> {
        let bounding_box_opt = obj.get_bounding_box();
        if bounding_box_opt.is_none() {
            return None;
        }
        let bounding_box = bounding_box_opt.unwrap();
        let mut cur_node = &self.nodes[&0];
        let mut next_node_idx = cur_node.find_fitting_child(&bounding_box);
        while !next_node_idx.is_none() {
            self.insert_node(next_node_idx.unwrap());
            cur_node = &self.nodes[&next_node_idx.unwrap()];
            next_node_idx = cur_node.find_fitting_child(&bounding_box);
        }
        return Some(cur_node.index);
    }
    fn new(x: CoordinateBorders, y: CoordinateBorders, z: CoordinateBorders) -> Self {
        Self {
            nodes: HashMap::from([(0, OctreeNode { index: 0, x, y, z })]),
        }
    }
    fn insert_node(&mut self, idx: u64) {
        if idx == 0 {
            panic!("überschreib mal nicht die rootnode! (insert_node mit index 0)")
        }
        if self.nodes.contains_key(&idx) {
            return;
        }
        let parent = OctreeNode::get_idx_parent(idx);
        match parent {
            Some(key) => {
                if !self.nodes.contains_key(&key) {
                    self.insert_node(key);
                }
            }
            None => unreachable!("warum hast du keine root node?"),
        }

        self.nodes.insert(
            idx,
            OctreeNode::from_pos_in_parent(
                idx,
                &OctreeNode::get_idx_pos_in_parent(idx).unwrap(),
                &self.nodes[&OctreeNode::get_idx_parent(idx).unwrap()],
            ),
        );
    }
}

struct OctreeNode {
    index: u64,
    x: CoordinateBorders,
    y: CoordinateBorders,
    z: CoordinateBorders,
}

#[derive(PartialEq, Eq)]
enum PosInParent {
    X0Y0Z0,
    X1Y0Z0,
    X0Y1Z0,
    X1Y1Z0,
    X0Y0Z1,
    X1Y0Z1,
    X0Y1Z1,
    X1Y1Z1,
}

impl PosInParent {
    fn in_lower_x_half(&self) -> bool {
        return *self == PosInParent::X0Y0Z0
            || *self == PosInParent::X0Y1Z0
            || *self == PosInParent::X0Y0Z1
            || *self == PosInParent::X0Y1Z1;
    }
    fn in_lower_y_half(&self) -> bool {
        return *self == PosInParent::X0Y0Z0
            || *self == PosInParent::X1Y0Z0
            || *self == PosInParent::X0Y0Z1
            || *self == PosInParent::X1Y0Z1;
    }
    fn in_lower_z_half(&self) -> bool {
        return *self == PosInParent::X0Y0Z0
            || *self == PosInParent::X1Y0Z0
            || *self == PosInParent::X0Y1Z0
            || *self == PosInParent::X1Y1Z0;
    }
    fn from_bools(in_lower_x_half: bool, in_lower_y_half: bool, in_lower_z_half: bool) -> Self {
        let mut array_pos = 0;
        if !in_lower_x_half {
            array_pos += 1;
        }
        if !in_lower_y_half {
            array_pos += 2;
        }
        if !in_lower_z_half {
            array_pos += 4;
        }
        Self::from_u32(array_pos).unwrap()
    }
    fn from_u32(array_pos: u32) -> Option<Self> {
        match array_pos {
            0 => Some(PosInParent::X0Y0Z0),
            1 => Some(PosInParent::X1Y0Z0),
            2 => Some(PosInParent::X0Y1Z0),
            3 => Some(PosInParent::X1Y1Z0),
            4 => Some(PosInParent::X0Y0Z1),
            5 => Some(PosInParent::X1Y0Z1),
            6 => Some(PosInParent::X0Y1Z1),
            7 => Some(PosInParent::X1Y1Z1),
            _ => unreachable!("child array too long"),
        }
    }
    fn get_idx(&self, parent_idx: u64) -> u64 {
        match *self {
            PosInParent::X0Y0Z0 => parent_idx * 8 + 1,
            PosInParent::X1Y0Z0 => parent_idx * 8 + 2,
            PosInParent::X0Y1Z0 => parent_idx * 8 + 3,
            PosInParent::X1Y1Z0 => parent_idx * 8 + 4,
            PosInParent::X0Y0Z1 => parent_idx * 8 + 5,
            PosInParent::X1Y0Z1 => parent_idx * 8 + 6,
            PosInParent::X0Y1Z1 => parent_idx * 8 + 7,
            PosInParent::X1Y1Z1 => parent_idx * 8 + 8,
        }
    }
}

impl OctreeNode {
    pub fn from_pos_in_parent(idx: u64, pos_in_parent: &PosInParent, parent: &OctreeNode) -> Self {
        Self {
            index: idx,
            x: CoordinateBorders::from_parent(&parent.x, pos_in_parent.in_lower_x_half()),
            y: CoordinateBorders::from_parent(&parent.y, pos_in_parent.in_lower_y_half()),
            z: CoordinateBorders::from_parent(&parent.z, pos_in_parent.in_lower_z_half()),
        }
    }
    pub fn find_fitting_child(&self, bounding_box: &BoundingBox) -> Option<u64> {
        let is_within_self = bounding_box.x.is_within(&self.x)
            && bounding_box.y.is_within(&self.y)
            && bounding_box.z.is_within(&self.z);
        if !is_within_self {
            return None;
        }
        let in_lower_x_half = bounding_box.x.is_within(&CoordinateBorders {
            lower: self.x.lower,
            upper: self.x.get_middle(),
        });
        let in_upper_x_half = bounding_box.x.is_within(&CoordinateBorders {
            lower: self.x.get_middle(),
            upper: self.x.upper,
        });
        let in_lower_y_half = bounding_box.y.is_within(&CoordinateBorders {
            lower: self.y.lower,
            upper: self.y.get_middle(),
        });
        let in_upper_y_half = bounding_box.y.is_within(&CoordinateBorders {
            lower: self.y.get_middle(),
            upper: self.y.upper,
        });
        let in_lower_z_half = bounding_box.z.is_within(&CoordinateBorders {
            lower: self.z.lower,
            upper: self.z.get_middle(),
        });
        let in_upper_z_half = bounding_box.z.is_within(&CoordinateBorders {
            lower: self.z.get_middle(),
            upper: self.z.upper,
        });
        if (!in_lower_x_half && !in_upper_x_half)
            || (!in_lower_y_half && !in_upper_y_half)
            || (!in_lower_z_half && !in_upper_z_half)
        {
            return None;
        }
        if !OctreeNode::has_children(self.index) {
            return None;
        }
        return Some(
            PosInParent::from_bools(in_lower_x_half, in_lower_y_half, in_lower_z_half)
                .get_idx(self.index),
        );
    }
    pub fn get_idx_parent(idx: u64) -> Option<u64> {
        if idx != 0 {
            return Some((idx - 1) / 8);
        } else {
            return None;
        }
    }
    pub fn has_children(idx: u64) -> bool {
        let lowest_node_cutoff = u64::MAX / 8 - 1;
        return idx <= lowest_node_cutoff;
    }

    pub fn get_idx_children(idx: u64) -> Vec<u64> {
        if OctreeNode::has_children(idx) {
            return (idx * 8 + 1..idx * 8 + 8).collect();
        } else {
            return vec![];
        }
    }
    pub fn get_idx_pos_in_parent(idx: u64) -> Option<PosInParent> {
        let sibling_array = OctreeNode::get_idx_children(OctreeNode::get_idx_parent(idx)?);
        let mut arr_pos = 0;
        for val in sibling_array {
            if val == idx {
                break;
            }
            arr_pos += 1;
        }
        PosInParent::from_u32(arr_pos)
    }
}

struct OctreeIterator<'a> {
    octree: &'a Octree,
    last_up: u64,
    cur: u64,
}

impl<'a> OctreeIterator<'a> {
    fn new(octree: &'a Octree, start: u64) -> Self {
        Self {
            octree: octree,
            last_up: 0, //stores origin of the last step up the Octree
            cur: start,
        }
    }
}

impl<'a> Iterator for OctreeIterator<'a> {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        for child in OctreeNode::get_idx_children(self.cur) {
            if self.octree.nodes.contains_key(&child) && self.last_up < child {
                self.cur = child;
                return Some(self.cur);
            }
        }
        self.last_up = self.cur;
        self.cur = OctreeNode::get_idx_parent(self.cur)?;
        return Some(self.cur);
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
            if check_collision(colliding_node.1, cur_node.1) {
                if cur_node.0 != MESH_GAMEOBJECT_ID {
                    let cur_collision_data: &mut ColData =
                        collisions.entry(cur_node.0).or_insert(ColData::new());

                    cur_collision_data.objects.push(colliding_node.0);
                    cur_collision_data
                        .directions
                        .push(get_collision_direction(colliding_node.1, cur_node.1));
                }
                if colliding_node.0 != MESH_GAMEOBJECT_ID {
                    let colliding_node_collision_data: &mut ColData =
                        collisions.entry(colliding_node.0).or_insert(ColData::new());

                    colliding_node_collision_data.objects.push(cur_node.0);
                    colliding_node_collision_data
                        .directions
                        .push(get_collision_direction(cur_node.1, colliding_node.1));
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

pub fn calculate_collisions(entities: &mut HashMap<u32, Box<dyn GameObject>>) -> () {
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
    for (index, col_data) in collisions {
        let game_obj_opt = entities.get_mut(&index);
        if game_obj_opt.is_some() {
            game_obj_opt.unwrap().give_collision_info(col_data.into());
        }
    }
}

fn check_collision(obj_1: &HitBox, obj_2: &HitBox) -> bool {
    return true;
}

fn check_line_triangle_collision(line: &HitBox, triangle: &HitBox) -> bool {
    let (triangle_point, triangle_vec1, triangle_vec2) = {
        if let Some((p, v1, v2)) = extract_triangle_data(triangle) {
            (p, v1, v2)
        } else {
            return false;
        }
    };
    let (line_point, line_vec) = {
        if let Some((p, v)) = extract_line_data(line) {
            (p, v)
        } else {
            return false;
        }
    };
    let matrix = cgmath::Matrix3::from_cols(triangle_vec1, triangle_vec2, line_vec);
    let inv_matrix = matrix.invert();
    if inv_matrix.is_none() {
        return false;
    }
    let inv_matrix = inv_matrix.unwrap();
    let b = triangle_point - line_point;
    let (beta, gamma, alpha) = {
        let res: Vector3<f32> = inv_matrix * b;
        (-res[0], -res[1], res[2])
    };

    return beta >= 0. && beta <= 1. && gamma >= 0. && gamma <= 1. && alpha >= 0. && alpha <= 1.;
}

fn extract_triangle_data(triangle: &HitBox) -> Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)> {
    if !matches!(triangle, HitBox::Triangle { .. }) {
        return None;
    }
    //Linear Equation matrix*x = b
    let (point1, point2, point3) = {
        if let HitBox::Triangle {
            point1,
            point2,
            point3,
        } = triangle
        {
            (point1, point2, point3)
        } else {
            unreachable!("kein dreieck obwohl gecheckt");
        }
    };
    let triangle_point = Vector3::new(point1[0], point1[1], point1[2]);

    let triangle_vec1: Vector3<f32> =
        Vector3::new(point2[0], point2[1], point2[2]) - triangle_point;
    let triangle_vec2: Vector3<f32> =
        Vector3::new(point3[0], point3[1], point3[2]) - triangle_point;
    return Some((triangle_point, triangle_vec1, triangle_vec2));
}

fn extract_line_data(line: &HitBox) -> Option<(Vector3<f32>, Vector3<f32>)> {
    if !matches!(line, HitBox::Line { .. }) {
        return None;
    }
    let (point, vec) = {
        if let HitBox::Line { point, vec } = line {
            (point, vec)
        } else {
            unreachable!("keine linie obwohl gecheckt");
        }
    };
    let line_point = Vector3::new(point[0], point[1], point[2]);
    let line_vec = Vector3::new(vec[0], vec[1], vec[2]);
    return Some((line_point, line_vec));
}

fn extract_sphere_data(sphere: &HitBox) -> Option<(Vector3<f32>, f32)> {
    if !matches!(sphere, HitBox::Sphere { .. }) {
        return None;
    }
    let (transform, radius) = {
        if let HitBox::Sphere { transform, radius } = sphere {
            (transform, radius)
        } else {
            unreachable!("kein dreieck obwohl gecheckt");
        }
    };
    let sphere_point = Vector3::new(transform.pos[0], transform.pos[1], transform.pos[2]);
    Some((sphere_point, *radius))
}

fn check_line_sphere_collision(line: &HitBox, sphere: &HitBox) -> bool {
    let (line_point, line_vec) = {
        if let Some((p, v)) = extract_line_data(line) {
            (p, v)
        } else {
            return false;
        }
    };
    let (sphere_point, radius) = {
        if let Some((p, r)) = extract_sphere_data(line) {
            (p, r)
        } else {
            return false;
        }
    };
    return false;
}

fn get_collision_direction(stationary: &HitBox, moved: &HitBox) -> Vector3<f32> {
    return Vector3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
}
