//! What weather stations report, from the reports Airmass publishes.

use alloc::vec::Vec;

use airmass_core::{
    FlightCategory, TextWeatherReport, TimedField, WeatherProductSnapshot, WeatherSnapshot,
    WeatherStationId,
};
use libm::hypot;
use world_core::{GeodeticPosition, LocalTangentPlane};

/// One text report in a snapshot, with the product that carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationReport<'a> {
    /// The product state the report was published in: its revision, source
    /// selection and lifecycle.
    pub product: &'a WeatherProductSnapshot,
    /// The report, each field with Airmass's own evidence.
    pub report: &'a TextWeatherReport,
}

impl<'a> StationReport<'a> {
    /// The station the report names, when the source supplied one.
    pub fn station(&self) -> Option<&'a WeatherStationId> {
        self.report
            .station_id()
            .as_present()
            .map(|field| &field.value)
    }

    /// The flight category Airmass derived for the report, with its evidence.
    pub fn flight_category(&self) -> Option<&'a TimedField<FlightCategory>> {
        self.report.flight_category().as_present()
    }
}

/// Every text report of every available product, in product identity order.
pub fn text_reports(snapshot: &WeatherSnapshot) -> impl Iterator<Item = StationReport<'_>> {
    snapshot.products().iter().flat_map(|product| {
        product
            .state()
            .available()
            .and_then(|available| available.typed_value().as_present())
            .and_then(|value| value.value.text_reports())
            .unwrap_or_default()
            .iter()
            .map(move |report| StationReport { product, report })
    })
}

/// The available reports that name `station`.
pub fn station_reports<'a>(
    snapshot: &'a WeatherSnapshot,
    station: &'a WeatherStationId,
) -> impl Iterator<Item = StationReport<'a>> {
    text_reports(snapshot).filter(move |report| report.station() == Some(station))
}

/// Where weather stations are.
///
/// Airmass publishes a station's identity, not its position. A consumer that
/// asks "which reports are near here" supplies the positions from the
/// navigation data it already holds; this crate keeps no station table.
pub trait StationLocator {
    /// The position of `station`, when known.
    fn locate(&self, station: &WeatherStationId) -> Option<GeodeticPosition>;
}

/// A report whose station a [`StationLocator`] placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocatedReport<'a> {
    /// The report and its product.
    pub report: StationReport<'a>,
    /// The station's position, from the locator.
    pub position: GeodeticPosition,
    /// Horizontal distance from the query center, meters.
    pub distance_m: f64,
}

/// The available reports from stations within `radius_m` of `center`,
/// nearest first.
///
/// A report whose station the locator does not know is left out: it cannot
/// be called near or far.
pub fn reports_near<'a>(
    snapshot: &'a WeatherSnapshot,
    locator: &impl StationLocator,
    center: GeodeticPosition,
    radius_m: f64,
) -> Vec<LocatedReport<'a>> {
    let level = |position: GeodeticPosition| GeodeticPosition {
        ellipsoidal_height_m: 0.0,
        ..position
    };
    let Ok(plane) = LocalTangentPlane::new(level(center)) else {
        return Vec::new();
    };
    let mut located: Vec<LocatedReport<'a>> = text_reports(snapshot)
        .filter_map(|report| {
            let position = locator.locate(report.station()?)?;
            let offset = plane.to_ned(level(position)).ok()?;
            let distance_m = hypot(offset.north_m, offset.east_m);
            (distance_m <= radius_m).then_some(LocatedReport {
                report,
                position,
                distance_m,
            })
        })
        .collect();
    located.sort_by(|a, b| a.distance_m.total_cmp(&b.distance_m));
    located
}

#[cfg(test)]
pub(crate) mod tests;
