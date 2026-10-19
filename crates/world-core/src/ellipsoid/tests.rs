use super::*;

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn the_wgs84_axes_are_the_published_ones() {
    // NIMA TR8350.2 table 3.3: b = 6356752.3142 m, e² = 0.00669437999014.
    assert!(close(WGS84.semi_minor_m(), 6_356_752.314_2, 1e-4));
    assert!(close(
        WGS84.eccentricity_squared(),
        0.006_694_379_990_14,
        1e-14
    ));
}

#[test]
fn cardinal_points_land_on_the_axes() {
    let equator = WGS84.to_ecef(0.0, 0.0, 0.0);
    assert!(close(equator.x_m, 6_378_137.0, 1e-6) && close(equator.y_m, 0.0, 1e-6));
    let east = WGS84.to_ecef(0.0, 90.0, 0.0);
    assert!(close(east.x_m, 0.0, 1e-6) && close(east.y_m, 6_378_137.0, 1e-6));
    let pole = WGS84.to_ecef(90.0, 0.0, 0.0);
    assert!(close(pole.z_m, 6_356_752.314_2, 1e-4) && close(pole.x_m, 0.0, 1e-6));
}

#[test]
fn latitude_45_matches_the_reference_ecef_point() {
    // The ellipsoid point at 45° N, 0° E: x = N cos φ, z = N (1 - e²) sin φ.
    let point = WGS84.to_ecef(45.0, 0.0, 0.0);
    assert!(close(point.x_m, 4_517_590.878_8, 1e-3), "{point:?}");
    assert!(close(point.z_m, 4_487_348.408_9, 1e-3), "{point:?}");
}

#[test]
fn ecef_round_trips_across_the_globe_and_heights() {
    for lat in [-90.0, -60.5, -1.0e-9, 0.0, 33.3, 47.397_741_9, 89.999] {
        for lon in [-180.0, -74.738, 0.0, 8.545_593_8, 179.9] {
            for height in [-430.0, 0.0, 488.0, 12_000.0, 400_000.0] {
                let (back_lat, back_lon, back_h) = WGS84.from_ecef(WGS84.to_ecef(lat, lon, height));
                assert!(
                    close(back_lat, lat, 1e-9),
                    "{lat} {lon} {height}: {back_lat}"
                );
                if lat.abs() < 90.0 {
                    assert!(
                        close(back_lon, lon, 1e-9),
                        "{lat} {lon} {height}: {back_lon}"
                    );
                }
                assert!(
                    close(back_h, height, 1e-4),
                    "{lat} {lon} {height}: {back_h}"
                );
            }
        }
    }
}
