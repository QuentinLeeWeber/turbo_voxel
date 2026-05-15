#[cfg(test)]
mod octree_node {
    use super::super::*;
    #[test]
    fn idx_mapping_test() {
        for i in 0..10000 {
            let children = OctreeNode::get_idx_children(i);
            for child in children {
                assert_eq!(i, OctreeNode::get_idx_parent(child).unwrap());
            }
        }
        assert_eq!(OctreeNode::get_idx_parent(0), None);
        assert!(OctreeNode::get_idx_children(u64::MAX).len() == 0);
    }
    #[test]
    fn pos_in_parent_mapping() {
        for i in 0..10000 {
            let children = OctreeNode::get_idx_children(i);
            assert_eq!(children.len(), 8);
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[0]).unwrap(),
                PosInParent::X0Y0Z0
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[1]).unwrap(),
                PosInParent::X1Y0Z0
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[2]).unwrap(),
                PosInParent::X0Y1Z0
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[3]).unwrap(),
                PosInParent::X1Y1Z0
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[4]).unwrap(),
                PosInParent::X0Y0Z1
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[5]).unwrap(),
                PosInParent::X1Y0Z1
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[6]).unwrap(),
                PosInParent::X0Y1Z1
            );
            assert_eq!(
                OctreeNode::get_idx_pos_in_parent(children[7]).unwrap(),
                PosInParent::X1Y1Z1
            );
        }
        assert_eq!(OctreeNode::get_idx_pos_in_parent(0), None);
    }
    #[test]
    fn from_pos_in_parent() {
        let parent = OctreeNode {
            index: 0,
            x: CoordinateBorders::new(0., 100.),
            y: CoordinateBorders::new(0., 100.),
            z: CoordinateBorders::new(0., 100.),
        };
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X0Y0Z0, &parent);
        assert_eq!(child.index, 1);
        assert_eq!(child.x.lower, 0.);
        assert_eq!(child.x.upper, 50.);
        assert_eq!(child.y.lower, 0.);
        assert_eq!(child.y.upper, 50.);
        assert_eq!(child.z.lower, 0.);
        assert_eq!(child.z.upper, 50.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X1Y0Z0, &parent);
        assert_eq!(child.index, 2);
        assert_eq!(child.x.lower, 50.);
        assert_eq!(child.x.upper, 100.);
        assert_eq!(child.y.lower, 0.);
        assert_eq!(child.y.upper, 50.);
        assert_eq!(child.z.lower, 0.);
        assert_eq!(child.z.upper, 50.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X0Y1Z0, &parent);
        assert_eq!(child.index, 3);
        assert_eq!(child.x.lower, 0.);
        assert_eq!(child.x.upper, 50.);
        assert_eq!(child.y.lower, 50.);
        assert_eq!(child.y.upper, 100.);
        assert_eq!(child.z.lower, 0.);
        assert_eq!(child.z.upper, 50.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X1Y1Z0, &parent);
        assert_eq!(child.index, 4);
        assert_eq!(child.x.lower, 50.);
        assert_eq!(child.x.upper, 100.);
        assert_eq!(child.y.lower, 50.);
        assert_eq!(child.y.upper, 100.);
        assert_eq!(child.z.lower, 0.);
        assert_eq!(child.z.upper, 50.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X0Y0Z1, &parent);
        assert_eq!(child.index, 5);
        assert_eq!(child.x.lower, 0.);
        assert_eq!(child.x.upper, 50.);
        assert_eq!(child.y.lower, 0.);
        assert_eq!(child.y.upper, 50.);
        assert_eq!(child.z.lower, 50.);
        assert_eq!(child.z.upper, 100.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X1Y0Z1, &parent);
        assert_eq!(child.index, 6);
        assert_eq!(child.x.lower, 50.);
        assert_eq!(child.x.upper, 100.);
        assert_eq!(child.y.lower, 0.);
        assert_eq!(child.y.upper, 50.);
        assert_eq!(child.z.lower, 50.);
        assert_eq!(child.z.upper, 100.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X0Y1Z1, &parent);
        assert_eq!(child.index, 7);
        assert_eq!(child.x.lower, 0.);
        assert_eq!(child.x.upper, 50.);
        assert_eq!(child.y.lower, 50.);
        assert_eq!(child.y.upper, 100.);
        assert_eq!(child.z.lower, 50.);
        assert_eq!(child.z.upper, 100.);
        let child = OctreeNode::from_pos_in_parent(&PosInParent::X1Y1Z1, &parent);
        assert_eq!(child.index, 8);
        assert_eq!(child.x.lower, 50.);
        assert_eq!(child.x.upper, 100.);
        assert_eq!(child.y.lower, 50.);
        assert_eq!(child.y.upper, 100.);
        assert_eq!(child.z.lower, 50.);
        assert_eq!(child.z.upper, 100.);
    }
    #[test]
    fn find_fitting_child() {
        let parent = OctreeNode {
            index: 0,
            x: CoordinateBorders::new(0., 100.),
            y: CoordinateBorders::new(0., 100.),
            z: CoordinateBorders::new(0., 100.),
        };
        let non_fitting_bounding_box = BoundingBox {
            x: CoordinateBorders::new(0., 101.),
            y: CoordinateBorders::new(0., 101.),
            z: CoordinateBorders::new(0., 101.),
        };
        assert_eq!(parent.find_fitting_child(&non_fitting_bounding_box), None);
        let x0y0z0_fitting_bounding_box = BoundingBox {
            x: CoordinateBorders::new(0., 49.),
            y: CoordinateBorders::new(0., 49.),
            z: CoordinateBorders::new(0., 49.),
        };
        assert_eq!(
            parent.find_fitting_child(&x0y0z0_fitting_bounding_box),
            Some(1)
        );
        let x1y0z0_fitting_bounding_box = BoundingBox {
            x: CoordinateBorders::new(51., 99.),
            y: CoordinateBorders::new(0., 49.),
            z: CoordinateBorders::new(0., 49.),
        };
        assert_eq!(
            parent.find_fitting_child(&x1y0z0_fitting_bounding_box),
            Some(2)
        );
    }
}
#[cfg(test)]
mod pos_in_parent {
    use super::super::*;
    #[test]
    fn u32_positions_as_expected() {
        /*
         * Expected division:
         * X -> left(0) to right(1),
         * Y -> bottom(0) to top(1),
         * Z -> front(0) to back(0)
         * front layer:
         * ________
         * | 2 | 3 |
         * |___|___|
         * | 0 | 1 |
         * |___|___|
         *
         * back layer:
         * ________
         * | 6 | 7 |
         * |___|___|
         * | 4 | 5 |
         * |___|___|
         */
        assert_eq!(PosInParent::from_u32(0).unwrap(), PosInParent::X0Y0Z0);
        assert_eq!(PosInParent::from_u32(1).unwrap(), PosInParent::X1Y0Z0);
        assert_eq!(PosInParent::from_u32(2).unwrap(), PosInParent::X0Y1Z0);
        assert_eq!(PosInParent::from_u32(3).unwrap(), PosInParent::X1Y1Z0);
        assert_eq!(PosInParent::from_u32(4).unwrap(), PosInParent::X0Y0Z1);
        assert_eq!(PosInParent::from_u32(5).unwrap(), PosInParent::X1Y0Z1);
        assert_eq!(PosInParent::from_u32(6).unwrap(), PosInParent::X0Y1Z1);
        assert_eq!(PosInParent::from_u32(7).unwrap(), PosInParent::X1Y1Z1);
        assert_eq!(PosInParent::from_u32(8), None);
    }
    #[test]
    fn from_bools_test() {
        assert_eq!(
            PosInParent::from_bools(true, true, true),
            PosInParent::X0Y0Z0
        );
        assert_eq!(
            PosInParent::from_bools(false, true, true),
            PosInParent::X1Y0Z0
        );
        assert_eq!(
            PosInParent::from_bools(true, false, true),
            PosInParent::X0Y1Z0
        );
        assert_eq!(
            PosInParent::from_bools(false, false, true),
            PosInParent::X1Y1Z0
        );
        assert_eq!(
            PosInParent::from_bools(true, true, false),
            PosInParent::X0Y0Z1
        );
        assert_eq!(
            PosInParent::from_bools(false, true, false),
            PosInParent::X1Y0Z1
        );
        assert_eq!(
            PosInParent::from_bools(true, false, false),
            PosInParent::X0Y1Z1
        );
        assert_eq!(
            PosInParent::from_bools(false, false, false),
            PosInParent::X1Y1Z1
        );
    }
    fn pos_in_parent_consistency(pos_in_parent: PosInParent) -> bool {
        return pos_in_parent
            == PosInParent::from_bools(
                pos_in_parent.in_lower_x_half(),
                pos_in_parent.in_lower_y_half(),
                pos_in_parent.in_lower_z_half(),
            );
    }
    #[test]
    fn consistency_bools() {
        assert!(pos_in_parent_consistency(PosInParent::X0Y0Z0));
        assert!(pos_in_parent_consistency(PosInParent::X1Y0Z0));
        assert!(pos_in_parent_consistency(PosInParent::X0Y1Z0));
        assert!(pos_in_parent_consistency(PosInParent::X1Y1Z0));
        assert!(pos_in_parent_consistency(PosInParent::X0Y0Z1));
        assert!(pos_in_parent_consistency(PosInParent::X1Y0Z1));
        assert!(pos_in_parent_consistency(PosInParent::X0Y1Z1));
        assert!(pos_in_parent_consistency(PosInParent::X1Y1Z1));
    }
    #[test]
    fn get_idx_correctness() {
        for i in 0..10000 {
            let children = OctreeNode::get_idx_children(i);
            for child in children {
                assert_eq!(
                    OctreeNode::get_idx_pos_in_parent(child).unwrap().get_idx(i),
                    child
                );
            }
        }
    }
}
