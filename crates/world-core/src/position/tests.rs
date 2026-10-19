use super::*;

#[test]
fn only_finite_coordinates_inside_the_latitude_range_are_well_formed() {
    assert!(GeodeticPosition::wgs84(90.0, -180.0, -400.0).is_well_formed());
    for bad in [
        GeodeticPosition::wgs84(90.5, 0.0, 0.0),
        GeodeticPosition::wgs84(f64::NAN, 0.0, 0.0),
        GeodeticPosition::wgs84(0.0, f64::INFINITY, 0.0),
        GeodeticPosition::wgs84(0.0, 0.0, f64::NAN),
    ] {
        assert!(!bad.is_well_formed(), "{bad:?}");
    }
}
