extern crate std;

use airmass_core::AdvisoryAltitudeReference;
use world_core::{GeodeticPosition, RouteError, RoutePoint, RouteSampling, WorldInstant};

use crate::advisory::tests::{advisory, band, delaware_box};
use crate::station::tests::{at, store};
use crate::{VerticalRelation, advisories_along_route};

fn point(latitude_deg: f64, height_m: f64, minutes: u64) -> RoutePoint {
    RoutePoint {
        position: GeodeticPosition::wgs84(latitude_deg, -75.45, height_m),
        at: WorldInstant::new(minutes * 60_000_000, minutes as i64 * 60_000_000),
    }
}

const EVERY_TEN_MINUTES: RouteSampling = RouteSampling {
    interval_micros: 600_000_000,
    max_samples: 16,
};

#[test]
fn a_route_meets_an_advisory_only_where_and_while_it_applies() {
    let msl = AdvisoryAltitudeReference::MeanSeaLevel;
    let mut store = store();
    // Valid for the first hour; the box spans 39.0 to 39.4 north.
    let sigmet = advisory(
        "sigmet/41c",
        Some(delaware_box()),
        Some(band((2_000, msl), (12_000, msl))),
    );
    store.accept(sigmet, at(1_000)).unwrap();
    let snapshot = store.capture();

    // Northbound at 2,000 m from south of the box to north of it in 40 min.
    let through = [point(38.75, 2_000.0, 0), point(39.55, 2_000.0, 40)];
    let samples =
        advisories_along_route(snapshot.snapshot(), &through, EVERY_TEN_MINUTES, 0.0).unwrap();
    let hit_counts: std::vec::Vec<_> = samples.iter().map(|sample| sample.hits.len()).collect();
    assert_eq!(hit_counts, [0, 0, 1, 1, 0]);
    assert_eq!(samples[2].hits[0].vertical, VerticalRelation::Inside);

    // The same track flown after the advisory's hour meets nothing.
    let later = [point(38.75, 2_000.0, 60), point(39.55, 2_000.0, 100)];
    let samples =
        advisories_along_route(snapshot.snapshot(), &later, EVERY_TEN_MINUTES, 0.0).unwrap();
    assert!(samples.iter().all(|sample| sample.hits.is_empty()));
}

#[test]
fn a_route_past_the_sample_bound_is_refused() {
    let store = store();
    let snapshot = store.capture();
    let long = [point(38.8, 2_000.0, 0), point(39.6, 2_000.0, 600)];
    let outcome = advisories_along_route(snapshot.snapshot(), &long, EVERY_TEN_MINUTES, 0.0);
    assert_eq!(
        outcome.map(|_| ()),
        Err(RouteError::TooManySamples {
            needed: 61,
            limit: 16
        })
    );
}
