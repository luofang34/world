extern crate std;

use std::string::String;
use std::vec::Vec;

use airmass_core::{
    AbsenceEvidence, AbsenceReason, AbsentField, DeliveryPath, EvaluationTime, FieldState,
    FlightCategory, MonotonicTime, ProducerInstanceId, ProductProvenance, QualityEvidence,
    SourceEpoch, SourceId, SourceProductRevision, SourceRef, SourceTime, StoreConfig,
    TextWeatherReport, TimeEvidence, TimedField, UtcInterval, UtcTime, WeatherIngress,
    WeatherOrigin, WeatherPayload, WeatherProduct, WeatherProductId, WeatherReportType,
    WeatherStationId, WeatherStore, WeatherValue,
};
use world_core::GeodeticPosition;

use crate::{StationLocator, reports_near, station_reports, text_reports};

pub(crate) fn absent<T>() -> FieldState<T> {
    FieldState::absent(AbsentField::new(
        AbsenceReason::NotObserved,
        AbsenceEvidence::Unknown,
    ))
}

/// A field as a FIS-B receiver would stamp it at monotonic time zero.
pub(crate) fn received<T>(value: T) -> TimedField<T> {
    TimedField::new(
        value,
        TimeEvidence::new(MonotonicTime::from_micros(0), SourceTime::Unknown),
        QualityEvidence::Unavailable,
        ProductProvenance::new(
            SourceRef::new(SourceId::new("uat-978"), SourceEpoch::new(1)),
            WeatherOrigin::LocalReceiver,
            DeliveryPath::new("fis-b"),
            None,
        ),
    )
}

pub(crate) fn at(utc_millis: i64) -> EvaluationTime {
    EvaluationTime::new(
        UtcTime::from_unix_millis(utc_millis),
        MonotonicTime::from_micros(utc_millis as u64 * 1_000),
    )
}

pub(crate) fn store() -> WeatherStore {
    WeatherStore::new(StoreConfig::default(), ProducerInstanceId::new(1)).unwrap()
}

/// A product carrying `value`, valid over `valid` when one is given.
pub(crate) fn product(id: &str, value: WeatherValue, valid: Option<UtcInterval>) -> WeatherIngress {
    WeatherIngress::product(
        WeatherProductId::new(id),
        SourceProductRevision::new(1),
        WeatherProduct::new(
            received(WeatherPayload::new("text/plain", Vec::new())),
            FieldState::present(received(value)),
            absent(),
            absent(),
            valid.map_or_else(absent, |interval| FieldState::present(received(interval))),
            absent(),
        ),
    )
}

fn metar(station: &str, text: &str, category: FlightCategory) -> TextWeatherReport {
    TextWeatherReport::new(
        FieldState::present(received(WeatherReportType::Metar)),
        FieldState::present(received(WeatherStationId::new(station))),
        absent(),
        received(String::from(text)),
        FieldState::present(received(category)),
    )
}

fn store_with_two_stations() -> WeatherStore {
    let mut store = store();
    let reports = Vec::from([
        metar(
            "KDOV",
            "KDOV 011453Z 03008KT 2SM BR OVC008",
            FlightCategory::Ifr,
        ),
        metar(
            "KILG",
            "KILG 011451Z 02006KT 10SM FEW050",
            FlightCategory::Vfr,
        ),
    ]);
    store
        .accept(
            product("metar/region-1", WeatherValue::TextReports(reports), None),
            at(1_000),
        )
        .unwrap();
    store
}

struct TwoStations;

impl StationLocator for TwoStations {
    fn locate(&self, station: &WeatherStationId) -> Option<GeodeticPosition> {
        match station.as_str() {
            "KDOV" => Some(GeodeticPosition::wgs84(39.13, -75.47, 9.0)),
            // KILG is deliberately unknown to this locator.
            _ => None,
        }
    }
}

#[test]
fn a_station_reads_its_report_with_airmass_evidence() {
    let store = store_with_two_stations();
    let snapshot = store.capture();
    assert_eq!(text_reports(snapshot.snapshot()).count(), 2);

    let dover = WeatherStationId::new("KDOV");
    let reports: Vec<_> = station_reports(snapshot.snapshot(), &dover).collect();
    assert_eq!(reports.len(), 1);
    let category = reports[0].flight_category().unwrap();
    assert_eq!(category.value, FlightCategory::Ifr);
    // The provenance is the one Airmass recorded at ingress, not a copy.
    assert_eq!(category.provenance.origin, WeatherOrigin::LocalReceiver);
    assert_eq!(category.provenance.source.id.as_str(), "uat-978");
    assert_eq!(reports[0].product.product_id().as_str(), "metar/region-1");
}

#[test]
fn nearby_reports_need_a_position_the_locator_knows() {
    let store = store_with_two_stations();
    let snapshot = store.capture();
    let here = GeodeticPosition::wgs84(39.18, -75.53, 500.0);

    let near = reports_near(snapshot.snapshot(), &TwoStations, here, 20_000.0);
    assert_eq!(near.len(), 1);
    assert_eq!(near[0].report.station().unwrap().as_str(), "KDOV");
    assert!(near[0].distance_m > 7_000.0 && near[0].distance_m < 8_000.0);

    assert!(reports_near(snapshot.snapshot(), &TwoStations, here, 5_000.0).is_empty());
}

#[test]
fn an_expired_product_no_longer_reports() {
    let mut store = store();
    let valid = UtcInterval::new(
        UtcTime::from_unix_millis(0),
        UtcTime::from_unix_millis(60_000),
    );
    let reports = Vec::from([metar("KDOV", "KDOV 011453Z", FlightCategory::Ifr)]);
    store
        .accept(
            product(
                "metar/kdov",
                WeatherValue::TextReports(reports),
                Some(valid),
            ),
            at(1_000),
        )
        .unwrap();
    assert_eq!(text_reports(store.capture().snapshot()).count(), 1);

    store.advance_time(at(60_000)).unwrap();
    assert_eq!(text_reports(store.capture().snapshot()).count(), 0);
}
