extern crate std;

use std::vec::Vec;

use surveillance_core::TrackSnapshotHandle;
use world_core::{GeodeticPosition, RouteError, RoutePoint, RouteSampling, WorldInstant};

use crate::fix::tests::{INTRUDER, SECOND, engine, report, tracked};
use crate::{RouteTrafficQuery, traffic_along_route};

fn leg(start_s: u64, end_s: u64) -> [RoutePoint; 2] {
    let at = |latitude_deg, seconds: u64| RoutePoint {
        position: GeodeticPosition::wgs84(latitude_deg, -75.0, 1_000.0),
        at: WorldInstant::new(seconds * SECOND, 0),
    };
    // Northbound at 0.0005 degrees of latitude per second, about 55 m/s.
    [
        at(40.0, start_s),
        at(40.0 + 0.0005 * (end_s - start_s) as f64, end_s),
    ]
}

const QUERY: RouteTrafficQuery = RouteTrafficQuery {
    sampling: RouteSampling {
        interval_micros: 5 * SECOND,
        max_samples: 16,
    },
    radius_m: 2_000.0,
};

fn one_track() -> Vec<TrackSnapshotHandle> {
    let mut engine = engine();
    // Reported at 1 s, 1 km east of the route's start, flying north with it.
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, (40.0, -74.9883), (107.0, 0.0)),
    );
    Vec::from([snapshot])
}

#[test]
fn the_near_part_of_a_route_sees_the_track_and_the_rest_says_unknown() {
    let tracks = one_track();
    let samples = traffic_along_route(&leg(1, 31), &tracks, QUERY).unwrap();
    assert_eq!(samples.len(), 7);

    // Inside the producer's 15 s window the track is placed beside the route.
    for sample in &samples[..3] {
        assert_eq!(sample.unplaced_tracks, 0);
        assert_eq!(sample.encounters.len(), 1);
        let range_m = sample.encounters[0].horizontal_range_m;
        assert!((range_m - 1_000.0).abs() < 60.0, "{range_m}");
    }
    assert_eq!(samples[0].encounters[0].fix.position_age_micros, 0);
    assert_eq!(
        samples[2].encounters[0].fix.position_age_micros,
        10 * SECOND
    );

    // Past it nothing is placed, and the sample says so instead of "clear".
    for sample in &samples[4..] {
        assert!(sample.encounters.is_empty());
        assert_eq!(sample.unplaced_tracks, 1);
    }
}

#[test]
fn a_track_outside_the_radius_is_placed_but_not_reported() {
    let tracks = one_track();
    let narrow = RouteTrafficQuery {
        radius_m: 500.0,
        ..QUERY
    };
    let samples = traffic_along_route(&leg(1, 11), &tracks, narrow).unwrap();
    assert!(samples.iter().all(|sample| sample.encounters.is_empty()));
    assert!(samples.iter().all(|sample| sample.unplaced_tracks == 0));
}

#[test]
fn a_route_past_the_sample_bound_is_refused() {
    let tracks = one_track();
    assert_eq!(
        traffic_along_route(&leg(1, 601), &tracks, QUERY),
        Err(RouteError::TooManySamples {
            needed: 121,
            limit: 16
        })
    );
}
