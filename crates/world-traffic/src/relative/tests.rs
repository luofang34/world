extern crate std;

use surveillance_core::{AddressNamespace, TrackKey};
use world_core::{GeodeticPosition, LocalTangentPlane, Ned};

use crate::fix::tests::{INTRUDER, SECOND, engine, report, tracked};
use crate::{AltitudeReference, Ownship, TrafficRefusal, relative_to_ownship};

const STILL: Ned = Ned {
    north_m: 0.0,
    east_m: 0.0,
    down_m: 0.0,
};

fn ownship(velocity_ned_m_s: Ned) -> Ownship {
    Ownship {
        position: GeodeticPosition::wgs84(40.0, -75.0, 1_000.0),
        velocity_ned_m_s,
        pressure_altitude_ft: Some(3_000.0),
    }
}

/// The latitude and longitude `north_m`, `east_m` from the test ownship.
fn offset_from_ownship(north_m: f64, east_m: f64) -> (f64, f64) {
    let plane = LocalTangentPlane::new(ownship(STILL).position).unwrap();
    let there = plane.to_geodetic(Ned {
        north_m,
        east_m,
        down_m: 0.0,
    });
    (there.latitude_deg, there.longitude_deg)
}

#[test]
fn a_head_on_track_closes_at_the_sum_of_both_speeds() {
    let mut engine = engine();
    let ahead = offset_from_ownship(6_000.0, 0.0);
    // 100 kt southbound is 51.44 m/s toward an ownship doing 50 m/s north.
    let snapshot = tracked(&mut engine, report(SECOND, INTRUDER, ahead, (100.0, 180.0)));
    let own = ownship(Ned {
        north_m: 50.0,
        ..STILL
    });

    let relative = relative_to_ownship(&own, &snapshot, SECOND).unwrap();
    assert!((relative.horizontal_range_m - 6_000.0).abs() < 1.0);
    assert!(
        relative.true_bearing_rad < 1e-3
            || relative.true_bearing_rad > core::f64::consts::TAU - 1e-3
    );
    let altitude = relative.relative_altitude.unwrap();
    assert_eq!(altitude.reference, AltitudeReference::Pressure);
    assert!((altitude.above_ownship_m - 152.4).abs() < 1e-9);

    let cpa = relative.cpa.unwrap();
    let closing = 50.0 + 100.0 * 1_852.0 / 3_600.0;
    assert!((cpa.closure_rate_m_s - closing).abs() < 1e-3);
    assert!((cpa.time_to_cpa_s - 6_000.0 / closing).abs() < 0.02);
    assert!(cpa.horizontal_miss_m < 1.0);
}

#[test]
fn a_crossing_track_misses_by_its_abeam_distance() {
    let mut engine = engine();
    // 4 km north and 3 km east, flying west: it passes 4 km north of a
    // stationary ownship after covering the 3 km.
    let crossing = offset_from_ownship(4_000.0, 3_000.0);
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, crossing, (100.0, 270.0)),
    );

    let relative = relative_to_ownship(&ownship(STILL), &snapshot, SECOND).unwrap();
    assert!((relative.horizontal_range_m - 5_000.0).abs() < 1.0);
    assert!((relative.north_m - 4_000.0).abs() < 1.0);
    assert!((relative.east_m - 3_000.0).abs() < 1.0);
    let cpa = relative.cpa.unwrap();
    let speed = 100.0 * 1_852.0 / 3_600.0;
    assert!((cpa.time_to_cpa_s - 3_000.0 / speed).abs() < 0.05);
    assert!((cpa.horizontal_miss_m - 4_000.0).abs() < 1.0);
    assert!((cpa.closure_rate_m_s - speed * 0.6).abs() < 0.01);
}

#[test]
fn a_departing_track_is_closest_now() {
    let mut engine = engine();
    let ahead = offset_from_ownship(6_000.0, 0.0);
    let snapshot = tracked(&mut engine, report(SECOND, INTRUDER, ahead, (100.0, 0.0)));
    let cpa = relative_to_ownship(&ownship(STILL), &snapshot, SECOND)
        .unwrap()
        .cpa
        .unwrap();
    assert_eq!(cpa.time_to_cpa_s, 0.0);
    assert!((cpa.horizontal_miss_m - 6_000.0).abs() < 1.0);
    assert!(cpa.closure_rate_m_s < 0.0);
}

#[test]
fn altitudes_on_different_references_are_never_subtracted() {
    let mut engine = engine();
    let ahead = offset_from_ownship(6_000.0, 0.0);
    let mut geometric_only = report(SECOND, INTRUDER, ahead, (100.0, 180.0));
    geometric_only.pressure_altitude_ft = None;
    geometric_only.geometric_altitude_ft = Some(5_000);
    let snapshot = tracked(&mut engine, geometric_only);

    // Ownship's pressure altitude has nothing to compare with, so the
    // geometric pair answers.
    let relative = relative_to_ownship(&ownship(STILL), &snapshot, SECOND).unwrap();
    let altitude = relative.relative_altitude.unwrap();
    assert_eq!(altitude.reference, AltitudeReference::Geometric);
    assert!((altitude.above_ownship_m - 524.0).abs() < 1e-9);

    let mut no_altitude = report(
        SECOND,
        TrackKey::new(AddressNamespace::Icao, 7),
        ahead,
        (100.0, 180.0),
    );
    no_altitude.pressure_altitude_ft = None;
    let bare = tracked(&mut engine, no_altitude);
    let relative = relative_to_ownship(&ownship(STILL), &bare, SECOND).unwrap();
    assert_eq!(relative.relative_altitude, None);
}

#[test]
fn a_track_without_velocity_has_geometry_but_no_closest_approach() {
    let mut engine = engine();
    let ahead = offset_from_ownship(6_000.0, 0.0);
    let mut parked = report(SECOND, INTRUDER, ahead, (0.0, 0.0));
    parked.velocity = None;
    let snapshot = tracked(&mut engine, parked);
    let relative = relative_to_ownship(&ownship(STILL), &snapshot, 5 * SECOND).unwrap();
    assert!((relative.horizontal_range_m - 6_000.0).abs() < 1.0);
    assert_eq!(relative.cpa, None);
}

#[test]
fn refusals_reach_the_caller() {
    let mut engine = engine();
    let ahead = offset_from_ownship(6_000.0, 0.0);
    let snapshot = tracked(&mut engine, report(SECOND, INTRUDER, ahead, (100.0, 180.0)));
    assert!(matches!(
        relative_to_ownship(&ownship(STILL), &snapshot, 30 * SECOND),
        Err(TrafficRefusal::Coasting { .. })
    ));

    let mut off_earth = ownship(STILL);
    off_earth.position.latitude_deg = 95.0;
    assert_eq!(
        relative_to_ownship(&off_earth, &snapshot, SECOND),
        Err(TrafficRefusal::ReferenceNotUsable)
    );

    engine.set_ownship_identities(&[INTRUDER], |_| {}).unwrap();
    let shadow = engine.snapshot(INTRUDER).unwrap();
    assert_eq!(
        relative_to_ownship(&ownship(STILL), &shadow, SECOND),
        Err(TrafficRefusal::OwnshipShadow { id: shadow.id })
    );
}
