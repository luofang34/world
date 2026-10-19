use super::*;

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

fn plane(lat: f64, lon: f64, height: f64) -> LocalTangentPlane {
    LocalTangentPlane::new(GeodeticPosition::wgs84(lat, lon, height)).expect("a valid origin")
}

#[test]
fn the_origin_is_the_zero_offset() {
    let frame = plane(47.397_741_9, 8.545_593_8, 488.0);
    let back = frame.to_geodetic(Ned {
        north_m: 0.0,
        east_m: 0.0,
        down_m: 0.0,
    });
    assert!(close(back.latitude_deg, 47.397_741_9, 1e-12));
    assert!(close(back.longitude_deg, 8.545_593_8, 1e-12));
    assert!(close(back.ellipsoidal_height_m, 488.0, 1e-6));
}

#[test]
fn one_degree_of_meridian_at_the_equator_matches_its_arc() {
    // The WGS-84 meridian arc from 0° to 1° is s = 110 574.39 m, and the
    // meridian radius of curvature at the equator is M = a(1 - e²) =
    // 6 335 439.33 m. The tangent plane holds the chord: its north part is
    // a few meters short of the arc, and the point lies below the plane by
    // about s² / 2M.
    let frame = plane(0.0, 0.0, 0.0);
    let ned = frame
        .to_ned(GeodeticPosition::wgs84(1.0, 0.0, 0.0))
        .expect("same datum");
    let arc = 110_574.39;
    assert!(ned.north_m < arc && arc - ned.north_m < 10.0, "{ned:?}");
    assert!(
        close(ned.down_m, arc * arc / (2.0 * 6_335_439.33), 1.0),
        "{ned:?}"
    );
    assert!(close(ned.east_m, 0.0, 1e-6), "{ned:?}");
}

#[test]
fn east_at_the_equator_follows_the_parallel() {
    // 0.001° of longitude on the equator is a·0.001·π/180 = 111.319 m.
    let frame = plane(0.0, 0.0, 0.0);
    let ned = frame
        .to_ned(GeodeticPosition::wgs84(0.0, 0.001, 0.0))
        .expect("same datum");
    assert!(close(ned.east_m, 111.319_490_8, 1e-3), "{ned:?}");
    assert!(ned.north_m.abs() < 1e-6 && ned.down_m > 0.0, "{ned:?}");
}

#[test]
fn up_is_minus_down_along_the_normal() {
    let frame = plane(40.577_186_1, -74.738_191_8, 56.3);
    let above = frame.to_geodetic(Ned {
        north_m: 0.0,
        east_m: 0.0,
        down_m: -1_000.0,
    });
    assert!(close(above.latitude_deg, 40.577_186_1, 1e-10));
    assert!(close(above.longitude_deg, -74.738_191_8, 1e-10));
    assert!(close(above.ellipsoidal_height_m, 1_056.3, 1e-6));
}

#[test]
fn offsets_round_trip_from_meters_to_hundreds_of_kilometers() {
    for (lat, lon) in [
        (47.397_741_9, 8.545_593_8),
        (-33.9, 151.2),
        (78.2, 15.6),
        (0.0, -179.99),
    ] {
        let frame = plane(lat, lon, 488.0);
        for offset in [
            Ned {
                north_m: 1.0,
                east_m: -2.0,
                down_m: 0.5,
            },
            Ned {
                north_m: 500.0,
                east_m: 1_200.0,
                down_m: -120.0,
            },
            Ned {
                north_m: -250_000.0,
                east_m: 180_000.0,
                down_m: 3_000.0,
            },
        ] {
            let back = frame.to_ned(frame.to_geodetic(offset)).expect("same datum");
            for (got, want) in [
                (back.north_m, offset.north_m),
                (back.east_m, offset.east_m),
                (back.down_m, offset.down_m),
            ] {
                assert!(close(got, want, 1e-6), "{lat} {lon} {offset:?}: {back:?}");
            }
        }
    }
}

#[test]
fn a_malformed_origin_or_position_is_refused() {
    assert_eq!(
        LocalTangentPlane::new(GeodeticPosition::wgs84(91.0, 0.0, 0.0)),
        Err(PositionError::NotWellFormed)
    );
    let frame = plane(0.0, 0.0, 0.0);
    assert_eq!(
        frame.to_ned(GeodeticPosition::wgs84(f64::NAN, 0.0, 0.0)),
        Err(PositionError::NotWellFormed)
    );
}
