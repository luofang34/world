use super::*;

#[test]
fn geometric_altitude_maps_to_a_lower_geopotential_one() {
    assert_eq!(Altitude::GeometricMsl(0.0).geopotential_m(), 0.0);
    // ISO 2533: 10 000 m geometric is 9 984.3 m geopotential.
    let h = Altitude::GeometricMsl(10_000.0).geopotential_m();
    assert!((h - 9_984.293).abs() < 1e-3, "{h}");
    assert_eq!(Altitude::Geopotential(10_000.0).geopotential_m(), 10_000.0);
}
