#[cfg(test)]
mod coordinate_borders {
    use crate::engine::physics::*;
    fn within_tol(a: f32, b: f32, atol: f32) -> bool {
        return a - atol < b && a + atol > b;
    }
    #[test]
    fn new_test() {
        let cord_bord = CoordinateBorders::new(10., 20.);
        assert!(cord_bord.upper == 20.);
        assert!(cord_bord.lower == 10.);
        //new ignores order of arguments
        let cord_bord = CoordinateBorders::new(20., 10.);
        assert!(cord_bord.upper == 20.);
        assert!(cord_bord.lower == 10.);
    }
    #[test]
    fn get_middle_test() {
        let cord_bord = CoordinateBorders::new(20., 10.);
        assert!(within_tol(cord_bord.get_middle(), 15., 0.0001));
    }
    #[test]
    fn from_parent_test() {
        let cord_bord = CoordinateBorders::new(20., 10.);
        let lower_half = CoordinateBorders::from_parent(&cord_bord, true);
        assert_eq!(lower_half.lower, 10.);
        assert!(within_tol(lower_half.upper, 15., 0.0001));
        let upper_half = CoordinateBorders::from_parent(&cord_bord, false);
        assert!(within_tol(upper_half.lower, 15., 0.0001));
        assert_eq!(upper_half.upper, 20.);
    }
    #[test]
    fn is_within_test() {
        let cord_bord = CoordinateBorders::new(20., 10.);
        let same_bord = CoordinateBorders::new(10., 20.);
        assert!(cord_bord.is_within(&same_bord));
        let intersecting_left_bord = CoordinateBorders::new(5., 15.);
        assert!(!cord_bord.is_within(&intersecting_left_bord));
        let intersecting_right_bord = CoordinateBorders::new(15., 25.);
        assert!(!cord_bord.is_within(&intersecting_right_bord));
        let containing_bord = CoordinateBorders::new(5., 25.);
        assert!(cord_bord.is_within(&containing_bord));
    }
}
