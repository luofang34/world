extern crate std;

use std::vec::Vec;

use crate::airmass::{
    AbsenceEvidence, AbsenceReason, AbsentField, FieldState, MonotonicTime, ProductProvenance,
    QualityEvidence, SourceEpoch, SourceId, SourceProductRevision, SourceTime, StoreError,
    TextWeatherReport, TimeEvidence, TimedField, UtcInterval, WeatherIngress, WeatherOrigin,
    WeatherPayload, WeatherProduct, WeatherProductId, WeatherProductState, WeatherReportType,
    WeatherValue,
};
use crate::aloft::{AloftLevel, SoundingProfile, WindModel};
use crate::atmosphere::{Altitude, AtmospherePoint};
use crate::surveillance::{
    AddressNamespace, DeliveryPath, ObservationOrigin, ObservationTime, PositionObservation,
    RemovalReason, SourceRef, TrackDelta, TrackKey, TrackPhase, TrafficObservation,
    VelocityObservation, Wgs84Position,
};
use crate::traffic::{Ownship, TrafficRefusal, relative_to_ownship};
use crate::weather::text_reports;
use crate::{
    DecimalYear, GeodeticPosition, MovingWorld, MovingWorldConfig, MovingWorldError, Ned,
    ValidityWindow, WorldInstant,
};

const SECOND: u64 = 1_000_000;
/// 2026-07-01T00:00:00Z, microseconds since the Unix epoch.
const START_UTC_MICROS: i64 = 1_782_864_000_000_000;
const INTRUDER: TrackKey = TrackKey::new(AddressNamespace::Icao, 0x00AC_82EC);

/// The instant `seconds` after a start where monotonic zero is `START_UTC`.
fn instant(seconds: u64) -> WorldInstant {
    WorldInstant::new(
        seconds * SECOND,
        START_UTC_MICROS + (seconds * SECOND) as i64,
    )
}

fn world(instance: u64) -> MovingWorld {
    MovingWorld::new(MovingWorldConfig::new(
        crate::surveillance::ProducerInstanceId::new(instance),
        crate::airmass::ProducerInstanceId::new(instance),
    ))
    .unwrap()
}

/// A simulator's truth report of one aircraft 6 km north of the test ownship.
fn simulated_traffic(at: WorldInstant) -> TrafficObservation {
    let mut observation = TrafficObservation::new(
        ObservationTime::local(at.monotonic_micros),
        SourceRef::new(7, 0, DeliveryPath::Simulator),
        ObservationOrigin::Replay,
        INTRUDER,
    );
    observation.position = Some(PositionObservation::Decoded(Wgs84Position {
        latitude_deg: 39.234,
        longitude_deg: -75.53,
    }));
    let mut velocity = VelocityObservation::default();
    velocity.ground_speed_kt = Some(100.0);
    velocity.track_angle_deg_true = Some(180.0);
    observation.velocity = Some(velocity);
    observation.pressure_altitude_ft = Some(3_500);
    observation
}

fn ownship() -> Ownship {
    Ownship {
        position: GeodeticPosition::wgs84(39.18, -75.53, 900.0),
        velocity_ned_m_s: Ned {
            north_m: 50.0,
            east_m: 0.0,
            down_m: 0.0,
        },
        pressure_altitude_ft: Some(3_000.0),
    }
}

fn received<T>(value: T) -> TimedField<T> {
    TimedField::new(
        value,
        TimeEvidence::new(MonotonicTime::from_micros(0), SourceTime::Unknown),
        QualityEvidence::Unavailable,
        ProductProvenance::new(
            crate::airmass::SourceRef::new(SourceId::new("uat-978"), SourceEpoch::new(1)),
            WeatherOrigin::LocalReceiver,
            crate::airmass::DeliveryPath::new("fis-b"),
            None,
        ),
    )
}

fn absent<T>() -> FieldState<T> {
    FieldState::absent(AbsentField::new(
        AbsenceReason::NotObserved,
        AbsenceEvidence::Unknown,
    ))
}

/// A winds-aloft text product valid for the first `valid_seconds`.
fn winds_aloft_product(id: &str, valid_seconds: u64) -> WeatherIngress {
    let report = TextWeatherReport::new(
        FieldState::present(received(WeatherReportType::WindsAndTemperaturesAloft)),
        absent(),
        absent(),
        received(std::string::String::from("FB ILG 3000 3615 6000 0120-02")),
        absent(),
    );
    let valid = UtcInterval::new(
        crate::clock::utc_time(instant(0)),
        crate::clock::utc_time(instant(valid_seconds)),
    );
    WeatherIngress::product(
        WeatherProductId::new(id),
        SourceProductRevision::new(1),
        WeatherProduct::new(
            received(WeatherPayload::new("text/plain", Vec::new())),
            FieldState::present(received(WeatherValue::TextReports(Vec::from([report])))),
            absent(),
            absent(),
            FieldState::present(received(valid)),
            absent(),
        ),
    )
}

/// The levels a host decoded from that product, as a sounding.
fn decoded_sounding(id: u64) -> SoundingProfile {
    SoundingProfile::new(
        id,
        None,
        0.0,
        ValidityWindow {
            from: DecimalYear(2026.0),
            until: DecimalYear(2027.0),
        },
        Vec::from([AloftLevel {
            altitude: Altitude::Geopotential(1_000.0),
            temperature_k: 285.0,
            pressure_pa: None,
            wind_ned_m_s: Ned {
                north_m: 14.0,
                east_m: -6.0,
                down_m: 0.0,
            },
            gust_m_s: 0.0,
            turbulence_edr: 0.0,
        }]),
    )
    .unwrap()
}

fn north_wind_at(world: &MovingWorld, at: WorldInstant) -> f64 {
    let point = AtmospherePoint {
        latitude_deg: 39.18,
        longitude_deg: -75.53,
        altitude: Altitude::Geopotential(1_000.0),
        at: at.decimal_year(),
    };
    world.aloft.sample(point).unwrap().wind_ned_m_s.north_m
}

#[test]
fn advancing_expires_a_track_and_a_weather_product_each_by_its_own_rule() {
    let mut world = world(1);
    world
        .surveillance
        .ingest(simulated_traffic(instant(1)), |_| {})
        .unwrap();
    world
        .accept_weather(winds_aloft_product("fb/ilg", 60), instant(1))
        .unwrap();

    // Queried through the umbrella while both are current.
    let track = world.surveillance.snapshot(INTRUDER).unwrap();
    let relative = relative_to_ownship(&ownship(), &track, instant(1).monotonic_micros).unwrap();
    assert!((relative.horizontal_range_m - 6_000.0).abs() < 20.0);
    assert!(relative.cpa.unwrap().time_to_cpa_s > 55.0);
    // Provenance is Surveillance's own, as ingested.
    let position = track.position.as_ref().unwrap();
    assert_eq!(position.provenance.source.delivery, DeliveryPath::Simulator);
    assert_eq!(position.provenance.origin, ObservationOrigin::Replay);
    assert_eq!(text_reports(world.weather.capture().snapshot()).count(), 1);

    // 30 s on: the track coasts and is refused; the product is still valid.
    let mut deltas = Vec::new();
    let report = world
        .advance(instant(30), |delta| deltas.push(delta.clone()))
        .unwrap();
    assert!(matches!(deltas.as_slice(), [TrackDelta::Coasting(_)]));
    assert!(report.weather.publication().is_none());
    let coasting = world.surveillance.snapshot(INTRUDER).unwrap();
    assert_eq!(coasting.phase, TrackPhase::Coasting);
    assert!(matches!(
        relative_to_ownship(&ownship(), &coasting, instant(30).monotonic_micros),
        Err(TrafficRefusal::Coasting { .. })
    ));

    // 62 s on: Surveillance removes the silent track, and Airmass publishes
    // the product as expired rather than dropping it.
    deltas.clear();
    let report = world
        .advance(instant(62), |delta| deltas.push(delta.clone()))
        .unwrap();
    assert!(matches!(
        deltas.as_slice(),
        [TrackDelta::Removed {
            reason: RemovalReason::Expired,
            ..
        }]
    ));
    assert!(world.surveillance.snapshot(INTRUDER).is_none());
    let published = report.weather.publication().unwrap();
    let product = published
        .snapshot()
        .product(&WeatherProductId::new("fb/ilg"))
        .unwrap();
    let WeatherProductState::Absent { absence } = product.state() else {
        panic!("an expired product is published as absent");
    };
    assert_eq!(absence.reason(), AbsenceReason::Expired);
    assert_eq!(text_reports(published.snapshot()).count(), 0);
    assert_eq!(world.now(), Some(instant(62)));
}

#[test]
fn a_clock_that_steps_back_is_refused_and_nothing_advances() {
    let mut world = world(2);
    world
        .surveillance
        .ingest(simulated_traffic(instant(1)), |_| {})
        .unwrap();
    world.advance(instant(10), |_| {}).unwrap();

    assert_eq!(
        world.advance(instant(9), |_| {}).map(|_| ()),
        Err(MovingWorldError::MonotonicRegression {
            previous_micros: 10 * SECOND,
            supplied_micros: 9 * SECOND,
        })
    );

    // UTC stepped back a minute while monotonic ran on: Airmass refuses, and
    // the engine was not advanced past its coast window by the refused call.
    let stepped = WorldInstant::new(40 * SECOND, START_UTC_MICROS - 60_000_000);
    let refused = world.advance(stepped, |_| {}).map(|_| ());
    assert!(matches!(
        refused,
        Err(MovingWorldError::Weather(
            StoreError::UtcTimeRegression { .. }
        ))
    ));
    assert_eq!(world.now(), Some(instant(10)));
    let track = world.surveillance.snapshot(INTRUDER).unwrap();
    assert_eq!(track.phase, TrackPhase::Active);
}

#[test]
fn a_sounding_bound_to_a_product_lives_exactly_as_long_as_the_product() {
    let mut world = world(3);
    let product_id = WeatherProductId::new("fb/ilg");
    assert_eq!(
        world.bind_sounding(&product_id, decoded_sounding(1)),
        Err(MovingWorldError::ProductNotAvailable {
            product_id: product_id.clone()
        })
    );

    world
        .accept_weather(winds_aloft_product("fb/ilg", 60), instant(1))
        .unwrap();
    world
        .bind_sounding(&product_id, decoded_sounding(1))
        .unwrap();
    assert_eq!(world.sounding_product(1), Some(&product_id));
    assert_eq!(north_wind_at(&world, instant(1)), 14.0);

    // The sounding's own window runs to 2027; Airmass's product ends at 60 s.
    let report = world.advance(instant(61), |_| {}).unwrap();
    assert_eq!(report.soundings_released, 1);
    assert_eq!(world.sounding_product(1), None);
    assert_eq!(north_wind_at(&world, instant(61)), 0.0);
}

#[test]
fn truth_and_belief_are_separate_worlds_fed_separately() {
    let mut truth = world(4);
    let mut belief = world(5);
    truth
        .surveillance
        .ingest(simulated_traffic(instant(1)), |_| {})
        .unwrap();
    truth.aloft.upsert_sounding(decoded_sounding(1)).unwrap();
    truth.advance(instant(2), |_| {}).unwrap();
    belief.advance(instant(2), |_| {}).unwrap();

    assert_eq!(truth.surveillance.track_count(), 1);
    assert_eq!(belief.surveillance.track_count(), 0);
    assert_eq!(north_wind_at(&truth, instant(2)), 14.0);
    assert_eq!(north_wind_at(&belief, instant(2)), 0.0);
    assert_ne!(
        truth.surveillance.producer_instance_id(),
        belief.surveillance.producer_instance_id()
    );
}
