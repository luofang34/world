use super::*;

fn gravity(lat: f64, height: f64) -> f64 {
    normal_gravity(GeodeticPosition::wgs84(lat, 0.0, height)).expect("a valid position")
}

#[test]
fn the_equator_and_poles_give_the_published_values() {
    // TR8350.2 table 3.4: γe = 9.7803253359, γp = 9.8321849378 m/s².
    assert!((gravity(0.0, 0.0) - 9.780_325_335_9).abs() < 1e-10);
    assert!((gravity(90.0, 0.0) - 9.832_184_937_8).abs() < 1e-9);
    assert!((gravity(-90.0, 0.0) - 9.832_184_937_8).abs() < 1e-9);
}

#[test]
fn somiglianas_constant_is_consistent_with_the_ellipsoid() {
    let e = crate::WGS84;
    let k = e.semi_minor_m() * 9.832_184_937_8 / (e.semi_major_m * 9.780_325_335_9) - 1.0;
    assert!((k - WGS84_NORMAL_GRAVITY.somigliana_k).abs() < 1e-11);
}

#[test]
fn gravity_rises_toward_the_poles() {
    let mut last = gravity(0.0, 0.0);
    for lat in [15.0, 30.0, 45.0, 60.0, 75.0, 90.0] {
        let now = gravity(lat, 0.0);
        assert!(now > last, "{lat}");
        last = now;
    }
    // Mid-latitude gravity sits near standard gravity, 9.80665 m/s².
    assert!((gravity(45.0, 0.0) - 9.806_65).abs() < 1e-3);
}

#[test]
fn the_free_air_gradient_is_about_3_086_micro_g_per_meter() {
    // The normal free-air gradient is 0.3086 mGal/m at the equator,
    // falling by about 0.1% toward the poles.
    for lat in [0.0, 45.0, 80.0] {
        let gradient = (gravity(lat, 1_000.0) - gravity(lat, 0.0)) / 1_000.0;
        assert!((gradient + 3.086e-6).abs() < 1.5e-8, "{lat}: {gradient}");
    }
}

#[test]
fn a_malformed_position_is_refused() {
    assert_eq!(
        normal_gravity(GeodeticPosition::wgs84(0.0, 0.0, f64::INFINITY)),
        Err(PositionError::NotWellFormed)
    );
}
