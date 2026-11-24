use world_core::{
    DecimalYear, GeodeticPosition, RouteError, RoutePoint, RouteSampling, ValidityWindow,
    WorldInstant,
};

use crate::model::tests::two_level_sounding;
use crate::{AloftModel, winds_along_route};

/// 2026-07-01T00:00:00Z, microseconds since the Unix epoch.
const DEPARTURE_UTC_MICROS: i64 = 1_782_864_000_000_000;

fn at(lat: f64, height_m: f64, seconds: u64) -> RoutePoint {
    RoutePoint {
        position: GeodeticPosition::wgs84(lat, -75.50, height_m),
        at: WorldInstant::new(
            seconds * 1_000_000,
            DEPARTURE_UTC_MICROS + seconds as i64 * 1_000_000,
        ),
    }
}

const EVERY_MINUTE: RouteSampling = RouteSampling {
    interval_micros: 60_000_000,
    max_samples: 16,
};

#[test]
fn a_climb_through_a_sounding_reads_its_winds_and_temperatures() {
    let mut model = AloftModel::default();
    let valid = ValidityWindow {
        from: DecimalYear(2026.0),
        until: DecimalYear(2027.0),
    };
    model.upsert_sounding(two_level_sounding(1, valid)).unwrap();

    let route = [at(39.18, 0.0, 0), at(39.22, 2_000.0, 120)];
    let samples = winds_along_route(&model, &route, EVERY_MINUTE).unwrap();
    assert_eq!(samples.len(), 3);
    let surface = samples[0].air.unwrap();
    let midway = samples[1].air.unwrap();
    let top = samples[2].air.unwrap();
    assert!((surface.wind_ned_m_s.north_m - 4.0).abs() < 1e-6);
    assert!(midway.wind_ned_m_s.north_m > 7.9 && midway.wind_ned_m_s.north_m < 8.1);
    assert!(top.wind_ned_m_s.north_m > 11.9);
    assert!(surface.state.temperature_k > midway.state.temperature_k);
    assert!(midway.state.temperature_k > top.state.temperature_k);
}

#[test]
fn a_route_flown_after_the_sounding_expires_reads_the_fallback() {
    let mut model = AloftModel::default();
    // Valid for the first minute of the flight only.
    let departure = WorldInstant::new(0, DEPARTURE_UTC_MICROS).decimal_year();
    let one_minute_in = WorldInstant::new(0, DEPARTURE_UTC_MICROS + 60_000_000).decimal_year();
    let valid = ValidityWindow {
        from: departure,
        until: one_minute_in,
    };
    model.upsert_sounding(two_level_sounding(1, valid)).unwrap();

    let route = [at(39.18, 1_000.0, 0), at(39.22, 1_000.0, 120)];
    let samples = winds_along_route(&model, &route, EVERY_MINUTE).unwrap();
    assert!(samples[0].air.unwrap().wind_ned_m_s.north_m > 7.0);
    assert_eq!(samples[2].air.unwrap().wind_ned_m_s.north_m, 0.0);
}

#[test]
fn a_route_past_the_sample_bound_is_refused() {
    let model = AloftModel::default();
    let route = [at(39.18, 0.0, 0), at(39.22, 0.0, 7_200)];
    assert_eq!(
        winds_along_route(&model, &route, EVERY_MINUTE),
        Err(RouteError::TooManySamples {
            needed: 121,
            limit: 16
        })
    );
}
