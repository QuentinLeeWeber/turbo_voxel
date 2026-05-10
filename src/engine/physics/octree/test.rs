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
    }
}
