use crate::engine::GameObject;
use std::collections::HashMap;

struct CoordinateBorders {
    upper: f32,
    lower: f32,
}

impl CoordinateBorders {
    pub fn get_middle(&self) -> f32 {
        return (self.upper + self.lower) / 2.;
    }
    pub fn new(upper: f32, lower: f32) -> Self {
        Self {
            upper: upper,
            lower: lower,
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
}

struct Octree {
    nodes: HashMap<u64, OctreeNode>,
}

impl Octree {
    pub fn add_element(&mut self, obj: crate::engine::HitBox) {}
    fn new(x: CoordinateBorders, y: CoordinateBorders, z: CoordinateBorders) -> Self {
        Self {
            nodes: HashMap::from([(0, OctreeNode { index: 0, x, y, z })]),
        }
    }
    fn insert_node(&mut self, idx: u64) {
        if idx == 0 {
            panic!("überschreib mal nicht die rootnode! (insert_node mit index 0)")
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
    pub fn find_fitting_child(&self) -> u64 {
        return self.index;
    }
    pub fn get_idx_parent(idx: u64) -> Option<u64> {
        if idx != 0 {
            return Some((idx - 1) / 8);
        } else {
            return None;
        }
    }
    pub fn get_idx_children(idx: u64) -> Vec<u64> {
        let lowest_node_cutoff = u64::MAX / 8 - 1;
        if idx <= lowest_node_cutoff {
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
        match arr_pos {
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
}

pub fn calculate_collision(entities: &mut HashMap<u32, Box<dyn GameObject>>) -> () {}
