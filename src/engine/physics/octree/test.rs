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
}
