extern crate std;

use surveillance_core::{
    AddressNamespace, Band, DeliveryPath, EngineConfig, ObservationOrigin, ObservationTime,
    PositionObservation, ProducerInstanceId, ProjectionBasis, SourceRef, SurveillanceEngine,
    TrackKey, TrackSnapshotHandle, TrafficObservation, VelocityObservation, Wgs84Position,
};

use crate::{TrafficRefusal, track_fix};

pub(crate) const SECOND: u64 = 1_000_000;
pub(crate) const INTRUDER: TrackKey = TrackKey::new(AddressNamespace::Icao, 0x00AC_82EC);

pub(crate) fn engine() -> SurveillanceEngine {
    SurveillanceEngine::new(EngineConfig::default(), ProducerInstanceId::new(1)).unwrap()
}

/// A direct ADS-B report: position, pressure altitude and ground velocity.
pub(crate) fn report(
    at_micros: u64,
    key: TrackKey,
    (latitude_deg, longitude_deg): (f64, f64),
    (ground_speed_kt, track_deg): (f64, f64),
) -> TrafficObservation {
    let mut observation = TrafficObservation::new(
        ObservationTime::local(at_micros),
        SourceRef::new(1, 0, DeliveryPath::LocalReceiver),
        ObservationOrigin::AdsbDirect {
            band: Band::Adsb1090,
        },
        key,
    );
    observation.position = Some(PositionObservation::Decoded(Wgs84Position {
        latitude_deg,
        longitude_deg,
    }));
    let mut velocity = VelocityObservation::default();
    velocity.ground_speed_kt = Some(ground_speed_kt);
    velocity.track_angle_deg_true = Some(track_deg);
    observation.velocity = Some(velocity);
    observation.pressure_altitude_ft = Some(3_500);
    observation
}

pub(crate) fn tracked(
    engine: &mut SurveillanceEngine,
    observation: TrafficObservation,
) -> TrackSnapshotHandle {
    let key = observation.key;
    engine.ingest(observation, |_| {}).unwrap();
    engine.snapshot(key).unwrap()
}

#[test]
fn a_reported_track_is_dead_reckoned_inside_its_bounds() {
    let mut engine = engine();
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, (40.0, -75.0), (120.0, 0.0)),
    );

    let reported = track_fix(&snapshot, SECOND).unwrap();
    assert_eq!(reported.basis, ProjectionBasis::Reported);
    assert_eq!(reported.position_age_micros, 0);
    assert_eq!(reported.position.latitude_deg, 40.0);

    let later = track_fix(&snapshot, 11 * SECOND).unwrap();
    assert_eq!(later.basis, ProjectionBasis::Extrapolated);
    assert_eq!(later.position_age_micros, 10 * SECOND);
    assert!(!later.distance_limited);
    // 120 kt for 10 s is 617.3 m north on Surveillance's sphere.
    let moved_m = (later.position.latitude_deg - 40.0).to_radians() * 6_371_008.8;
    assert!((moved_m - 617.3).abs() < 0.5, "{moved_m}");
    assert_eq!(later.pressure_altitude_ft, Some(3_500));
}

#[test]
fn a_fast_track_is_flagged_when_the_leg_reaches_the_distance_limit() {
    let mut engine = engine();
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, (40.0, -75.0), (480.0, 0.0)),
    );
    let fix = track_fix(&snapshot, 11 * SECOND).unwrap();
    assert!(fix.distance_limited);
    let moved_m = (fix.position.latitude_deg - 40.0).to_radians() * 6_371_008.8;
    assert!((moved_m - 1_000.0).abs() < 0.5, "{moved_m}");
}

#[test]
fn a_track_silent_past_the_coast_window_is_refused() {
    let mut engine = engine();
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, (40.0, -75.0), (120.0, 0.0)),
    );
    assert_eq!(
        track_fix(&snapshot, 19 * SECOND),
        Err(TrafficRefusal::Coasting {
            id: snapshot.id,
            silent_micros: 18 * SECOND,
            coast_after_micros: 15 * SECOND,
        })
    );

    // The engine reaches the same verdict when its own clock gets there.
    engine.advance_time(19 * SECOND, |_| {}).unwrap();
    let coasting = engine.snapshot(INTRUDER).unwrap();
    assert!(matches!(
        track_fix(&coasting, 19 * SECOND),
        Err(TrafficRefusal::Coasting { .. })
    ));
}

#[test]
fn a_position_older_than_the_projection_window_is_refused() {
    let mut config = EngineConfig::default();
    config.freshness.extrapolate_max_micros = 5 * SECOND;
    let mut engine = SurveillanceEngine::new(config, ProducerInstanceId::new(2)).unwrap();
    let snapshot = tracked(
        &mut engine,
        report(SECOND, INTRUDER, (40.0, -75.0), (120.0, 0.0)),
    );
    assert!(track_fix(&snapshot, 6 * SECOND).is_ok());
    assert_eq!(
        track_fix(&snapshot, 8 * SECOND),
        Err(TrafficRefusal::PositionStale {
            id: snapshot.id,
            age_micros: 7 * SECOND,
            limit_micros: 5 * SECOND,
        })
    );
}

#[test]
fn an_instant_before_the_observation_is_refused() {
    let mut engine = engine();
    let snapshot = tracked(
        &mut engine,
        report(4 * SECOND, INTRUDER, (40.0, -75.0), (120.0, 0.0)),
    );
    assert_eq!(
        track_fix(&snapshot, SECOND),
        Err(TrafficRefusal::BeforeObservation {
            id: snapshot.id,
            early_micros: 3 * SECOND,
        })
    );
}

#[test]
fn a_track_without_a_position_is_refused() {
    let mut engine = engine();
    let mut identity_only = report(SECOND, INTRUDER, (40.0, -75.0), (120.0, 0.0));
    identity_only.position = None;
    let snapshot = tracked(&mut engine, identity_only);
    assert_eq!(
        track_fix(&snapshot, SECOND),
        Err(TrafficRefusal::NoPosition { id: snapshot.id })
    );
}
