extern crate std;

use std::vec::Vec;

use airmass_core::{
    AdvisoryAltitude, AdvisoryAltitudeBand, AdvisoryAltitudeReference, AdvisoryGeometry,
    AdvisoryLine, AdvisoryPolygon, AdvisoryPosition, AdvisoryRing, AdvisoryValidity, FieldState,
    UtcInterval, UtcTime, WeatherAdvisory, WeatherAdvisoryId, WeatherAdvisoryType, WeatherIngress,
    WeatherStore, WeatherValue,
};
use world_core::GeodeticPosition;

use crate::station::tests::{absent, at, product, received, store};
use crate::{
    AdvisoryQuery, HorizontalRelation, QueryAltitude, VerticalRelation, advisories, advisories_at,
};

pub(crate) const VALID: UtcInterval = UtcInterval::new(
    UtcTime::from_unix_millis(0),
    UtcTime::from_unix_millis(3_600_000),
);

fn vertex(latitude_deg: f64, longitude_deg: f64) -> AdvisoryPosition {
    AdvisoryPosition::new((latitude_deg * 1e6) as i32, (longitude_deg * 1e6) as i32).unwrap()
}

/// The box 39.0..39.4 N, 75.7..75.2 W.
pub(crate) fn delaware_box() -> AdvisoryGeometry {
    let corners = [
        vertex(39.0, -75.7),
        vertex(39.0, -75.2),
        vertex(39.4, -75.2),
        vertex(39.4, -75.7),
        vertex(39.0, -75.7),
    ];
    let ring = AdvisoryRing::new(Vec::from(corners)).unwrap();
    AdvisoryGeometry::Polygon(AdvisoryPolygon::new(Vec::from([ring])).unwrap())
}

pub(crate) fn band(
    base: (u32, AdvisoryAltitudeReference),
    top: (u32, AdvisoryAltitudeReference),
) -> AdvisoryAltitudeBand {
    AdvisoryAltitudeBand::new(
        AdvisoryAltitude::new(base.0, base.1),
        AdvisoryAltitude::new(top.0, top.1),
    )
    .unwrap()
}

pub(crate) fn advisory(
    id: &str,
    geometry: Option<AdvisoryGeometry>,
    altitude_band: Option<AdvisoryAltitudeBand>,
) -> WeatherIngress {
    let advisory = WeatherAdvisory::new(
        FieldState::present(received(WeatherAdvisoryId::new(id))),
        FieldState::present(received(WeatherAdvisoryType::ConvectiveSigmet)),
        FieldState::present(received(AdvisoryValidity::Period(VALID))),
        geometry.map_or_else(absent, |value| FieldState::present(received(value))),
        absent(),
        altitude_band.map_or_else(absent, |value| FieldState::present(received(value))),
    );
    product(
        id,
        WeatherValue::Advisories(Vec::from([advisory])),
        Some(VALID),
    )
}

fn store_with(ingress: impl IntoIterator<Item = WeatherIngress>) -> WeatherStore {
    let mut store = store();
    for item in ingress {
        store.accept(item, at(1_000)).unwrap();
    }
    store
}

fn query(latitude_deg: f64, altitude: QueryAltitude, utc_millis: i64) -> AdvisoryQuery {
    AdvisoryQuery {
        position: GeodeticPosition::wgs84(latitude_deg, -75.45, 0.0),
        altitude,
        at: UtcTime::from_unix_millis(utc_millis),
        line_half_width_m: 10_000.0,
    }
}

const MSL: AdvisoryAltitudeReference = AdvisoryAltitudeReference::MeanSeaLevel;
const FL: AdvisoryAltitudeReference = AdvisoryAltitudeReference::FlightLevel;

fn at_msl(feet: f64) -> QueryAltitude {
    QueryAltitude {
        msl_ft: Some(feet),
        ..QueryAltitude::default()
    }
}

#[test]
fn a_point_inside_the_polygon_band_and_validity_is_a_hit() {
    let sigmet = advisory(
        "sigmet/41c",
        Some(delaware_box()),
        Some(band((2_000, MSL), (12_000, MSL))),
    );
    let store = store_with([sigmet]);
    let snapshot = store.capture();

    let hits = advisories_at(snapshot.snapshot(), &query(39.2, at_msl(6_000.0), 60_000));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].horizontal, HorizontalRelation::InsidePolygon);
    assert_eq!(hits[0].vertical, VerticalRelation::Inside);
    assert_eq!(
        hits[0].published.product.product_id().as_str(),
        "sigmet/41c"
    );

    // Outside horizontally, above the top, and after the validity ends.
    let outside = [
        query(39.6, at_msl(6_000.0), 60_000),
        query(39.2, at_msl(13_000.0), 60_000),
        query(39.2, at_msl(1_000.0), 60_000),
        query(39.2, at_msl(6_000.0), 3_600_000),
    ];
    for miss in outside {
        assert!(advisories_at(snapshot.snapshot(), &miss).is_empty());
    }
}

#[test]
fn a_band_on_a_reference_the_query_lacks_is_unresolved_not_dropped() {
    let airmet = advisory(
        "airmet/ice",
        Some(delaware_box()),
        Some(band((8_000, MSL), (18_000, FL))),
    );
    let store = store_with([airmet]);
    let snapshot = store.capture();

    let hits = advisories_at(snapshot.snapshot(), &query(39.2, at_msl(9_000.0), 60_000));
    assert_eq!(hits[0].vertical, VerticalRelation::Unresolved { needs: FL });

    let with_pressure = QueryAltitude {
        pressure_altitude_ft: Some(9_200.0),
        ..at_msl(9_000.0)
    };
    let hits = advisories_at(snapshot.snapshot(), &query(39.2, with_pressure, 60_000));
    assert_eq!(hits[0].vertical, VerticalRelation::Inside);

    // Known below the base: outside whatever the top is stated on.
    let below = advisories_at(snapshot.snapshot(), &query(39.2, at_msl(7_000.0), 60_000));
    assert!(below.is_empty());
}

#[test]
fn a_surface_based_band_needs_no_height_above_ground_for_its_base() {
    let surface = AdvisoryAltitudeReference::Surface;
    let airmet = advisory(
        "airmet/ifr",
        Some(delaware_box()),
        Some(band((0, surface), (4_000, MSL))),
    );
    let store = store_with([airmet]);
    let snapshot = store.capture();
    let hits = advisories_at(snapshot.snapshot(), &query(39.2, at_msl(1_500.0), 60_000));
    assert_eq!(hits[0].vertical, VerticalRelation::Inside);
}

#[test]
fn a_line_advisory_applies_within_the_stated_half_width() {
    let line = AdvisoryLine::new(Vec::from([vertex(39.0, -75.45), vertex(40.0, -75.45)])).unwrap();
    let store = store_with([advisory(
        "cwa/line",
        Some(AdvisoryGeometry::Polyline(line)),
        None,
    )]);
    let snapshot = store.capture();

    let mut near = query(39.5, QueryAltitude::default(), 60_000);
    near.position.longitude_deg = -75.40;
    let hits = advisories_at(snapshot.snapshot(), &near);
    assert_eq!(hits[0].vertical, VerticalRelation::Unstated);
    let HorizontalRelation::NearLine { distance_m } = hits[0].horizontal else {
        panic!("a line advisory relates by distance");
    };
    // 0.05 degrees of longitude at 39.5 N.
    assert!((distance_m - 4_290.0).abs() < 20.0, "{distance_m}");

    near.line_half_width_m = 2_000.0;
    assert!(advisories_at(snapshot.snapshot(), &near).is_empty());
}

#[test]
fn an_advisory_without_geometry_is_listed_but_never_placed() {
    let store = store_with([advisory("sigmet/text-only", None, None)]);
    let snapshot = store.capture();
    assert_eq!(advisories(snapshot.snapshot()).count(), 1);
    let anywhere = query(39.2, at_msl(6_000.0), 60_000);
    assert!(advisories_at(snapshot.snapshot(), &anywhere).is_empty());
}
