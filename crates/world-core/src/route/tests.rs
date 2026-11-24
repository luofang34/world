extern crate std;

use std::vec::Vec;

use crate::{GeodeticPosition, RouteError, RoutePoint, RouteSampling, WorldInstant, sample_route};

fn point(lat: f64, lon: f64, height_m: f64, seconds: u64) -> RoutePoint {
    RoutePoint {
        position: GeodeticPosition::wgs84(lat, lon, height_m),
        at: WorldInstant::new(seconds * 1_000_000, 1_000 + seconds as i64 * 1_000_000),
    }
}

const EVERY_TEN_SECONDS: RouteSampling = RouteSampling {
    interval_micros: 10_000_000,
    max_samples: 64,
};

#[test]
fn samples_fall_on_the_interval_and_end_on_the_last_point() {
    let route = [
        point(40.0, -75.0, 1_000.0, 0),
        point(41.0, -74.0, 2_000.0, 25),
    ];
    let samples: Vec<_> = sample_route(&route, EVERY_TEN_SECONDS).unwrap().collect();
    let seconds: Vec<_> = samples
        .iter()
        .map(|sample| sample.at.monotonic_micros / 1_000_000)
        .collect();
    assert_eq!(seconds, [0, 10, 20, 25]);
    assert_eq!(samples[3], route[1]);
    assert!((samples[1].position.latitude_deg - 40.4).abs() < 1e-12);
    assert!((samples[1].position.longitude_deg + 74.6).abs() < 1e-12);
    assert!((samples[1].position.ellipsoidal_height_m - 1_400.0).abs() < 1e-9);
    assert_eq!(samples[1].at.utc_unix_micros, 1_000 + 10_000_000);
}

#[test]
fn samples_cross_legs_and_the_antimeridian_the_short_way() {
    let route = [
        point(10.0, 179.0, 0.0, 0),
        point(10.0, -179.0, 0.0, 20),
        point(12.0, -179.0, 0.0, 40),
    ];
    let samples: Vec<_> = sample_route(&route, EVERY_TEN_SECONDS).unwrap().collect();
    assert_eq!(samples.len(), 5);
    assert!((samples[1].position.longitude_deg.abs() - 180.0).abs() < 1e-9);
    assert!((samples[3].position.latitude_deg - 11.0).abs() < 1e-12);
}

#[test]
fn a_single_point_is_one_sample() {
    let route = [point(40.0, -75.0, 0.0, 7)];
    let samples: Vec<_> = sample_route(&route, EVERY_TEN_SECONDS).unwrap().collect();
    assert_eq!(samples, route);
}

#[test]
fn a_route_past_the_sample_bound_is_refused_not_thinned() {
    let route = [point(40.0, -75.0, 0.0, 0), point(41.0, -75.0, 0.0, 3_600)];
    let outcome = sample_route(&route, EVERY_TEN_SECONDS).map(|_| ());
    assert_eq!(
        outcome,
        Err(RouteError::TooManySamples {
            needed: 361,
            limit: 64
        })
    );
}

#[test]
fn malformed_routes_are_refused() {
    let sampling = EVERY_TEN_SECONDS;
    assert_eq!(
        sample_route(&[], sampling).map(|_| ()),
        Err(RouteError::Empty)
    );
    let backwards = [point(40.0, -75.0, 0.0, 10), point(41.0, -75.0, 0.0, 10)];
    assert_eq!(
        sample_route(&backwards, sampling).map(|_| ()),
        Err(RouteError::TimeNotIncreasing { index: 1 })
    );
    let off_earth = [point(40.0, -75.0, 0.0, 0), point(95.0, -75.0, 0.0, 10)];
    assert_eq!(
        sample_route(&off_earth, sampling).map(|_| ()),
        Err(RouteError::PointNotUsable { index: 1 })
    );
    let zero = RouteSampling {
        interval_micros: 0,
        max_samples: 8,
    };
    assert_eq!(
        sample_route(&[point(40.0, -75.0, 0.0, 0)], zero).map(|_| ()),
        Err(RouteError::ZeroInterval)
    );
}
