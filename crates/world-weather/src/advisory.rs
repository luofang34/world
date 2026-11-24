//! Which advisories apply at a point, from the advisories Airmass publishes.

use alloc::vec::Vec;

use airmass_core::{
    AdvisoryAltitude, AdvisoryAltitudeBand, AdvisoryAltitudeReference, AdvisoryGeometry, UtcTime,
    WeatherAdvisory, WeatherProductSnapshot, WeatherSnapshot,
};
use world_core::GeodeticPosition;

mod footprint;

/// The altitudes a query can be compared on.
///
/// Advisory limits are stated above mean sea level, above ground, as flight
/// levels or at the surface. No one of these converts to another without
/// terrain and a pressure setting, so the caller gives each altitude it
/// actually has.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QueryAltitude {
    /// Altitude above mean sea level, feet.
    pub msl_ft: Option<f64>,
    /// Pressure altitude, feet; what a flight level is compared with.
    pub pressure_altitude_ft: Option<f64>,
    /// Height above ground level, feet.
    pub agl_ft: Option<f64>,
}

/// A place, altitude and time to ask advisories about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdvisoryQuery {
    /// Horizontal position; the height is not read.
    pub position: GeodeticPosition,
    /// The altitudes the caller has.
    pub altitude: QueryAltitude,
    /// The UTC time asked about, which an advisory's validity must contain.
    pub at: UtcTime,
    /// Distance from a line advisory inside which it applies, meters. A line
    /// has no inside, so the caller states how near counts.
    pub line_half_width_m: f64,
}

/// How a query position relates to an advisory's footprint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HorizontalRelation {
    /// Inside the advisory's polygon.
    InsidePolygon,
    /// Within the query's half-width of the advisory's line.
    NearLine {
        /// Distance to the line, meters.
        distance_m: f64,
    },
}

/// How a query altitude relates to an advisory's vertical extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalRelation {
    /// Between the advisory's base and top.
    Inside,
    /// The advisory states no altitude band.
    Unstated,
    /// A limit is on a reference the query gave no altitude for, so the
    /// query may be inside or outside.
    Unresolved {
        /// The reference the undecided limit is stated on.
        needs: AdvisoryAltitudeReference,
    },
}

/// One advisory in a snapshot, with the product that carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PublishedAdvisory<'a> {
    /// The product state the advisory was published in: its revision,
    /// source selection and lifecycle.
    pub product: &'a WeatherProductSnapshot,
    /// The advisory, each field with Airmass's own evidence.
    pub advisory: &'a WeatherAdvisory,
}

/// An advisory that applies, or may apply, to a query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdvisoryHit<'a> {
    /// The advisory and its product.
    pub published: PublishedAdvisory<'a>,
    /// How the query position falls on the advisory's footprint.
    pub horizontal: HorizontalRelation,
    /// How the query altitude falls in the advisory's vertical extent.
    pub vertical: VerticalRelation,
}

/// Every advisory of every available product, in product identity order.
///
/// This includes advisories no point query can place, such as one published
/// without geometry.
pub fn advisories(snapshot: &WeatherSnapshot) -> impl Iterator<Item = PublishedAdvisory<'_>> {
    snapshot.products().iter().flat_map(|product| {
        product
            .state()
            .available()
            .and_then(|available| available.typed_value().as_present())
            .and_then(|value| value.value.advisories())
            .unwrap_or_default()
            .iter()
            .map(move |advisory| PublishedAdvisory { product, advisory })
    })
}

/// The advisories that apply, or may apply, at `query`.
///
/// An advisory is returned when its validity contains `query.at`, its
/// footprint contains or passes near the position, and its altitude band is
/// not known to exclude the altitude. An advisory that definitely does not
/// apply is left out; one that cannot be decided vertically is returned as
/// [`VerticalRelation::Unresolved`], never dropped.
///
/// The snapshot holds what was known when it was captured. A product Airmass
/// had not made available by then, such as one not yet effective, is not in
/// it, whatever `query.at` asks.
pub fn advisories_at<'a>(
    snapshot: &'a WeatherSnapshot,
    query: &AdvisoryQuery,
) -> Vec<AdvisoryHit<'a>> {
    advisories(snapshot)
        .filter_map(|published| {
            let advisory = published.advisory;
            let validity = advisory.validity().as_present()?.value;
            if !validity.lifecycle_interval().ok()?.contains(query.at) {
                return None;
            }
            let horizontal = match &advisory.geometry().as_present()?.value {
                AdvisoryGeometry::Polygon(polygon) => {
                    footprint::inside_polygon(polygon, query.position)
                        .then_some(HorizontalRelation::InsidePolygon)?
                }
                AdvisoryGeometry::Polyline(line) => {
                    let distance_m = footprint::distance_to_line_m(line, query.position)?;
                    (distance_m <= query.line_half_width_m)
                        .then_some(HorizontalRelation::NearLine { distance_m })?
                }
                _ => return None,
            };
            let vertical = match advisory.altitude_band().as_present() {
                Some(band) => vertical_relation(band.value, query.altitude)?,
                None => VerticalRelation::Unstated,
            };
            Some(AdvisoryHit {
                published,
                horizontal,
                vertical,
            })
        })
        .collect()
}

/// `None` when the altitude is known to be outside the band.
fn vertical_relation(
    band: AdvisoryAltitudeBand,
    altitude: QueryAltitude,
) -> Option<VerticalRelation> {
    let above_base = match band.base().reference() {
        // Nothing is below the surface, so a surface base bounds nothing.
        AdvisoryAltitudeReference::Surface => Some(true),
        _ => on_reference(band.base(), altitude).map(|own| own >= f64::from(band.base().feet())),
    };
    let below_top = on_reference(band.top(), altitude).map(|own| own <= limit_feet(band.top()));
    match (above_base, below_top) {
        (Some(false), _) | (_, Some(false)) => None,
        (Some(true), Some(true)) => Some(VerticalRelation::Inside),
        (None, _) => Some(VerticalRelation::Unresolved {
            needs: band.base().reference(),
        }),
        (_, None) => Some(VerticalRelation::Unresolved {
            needs: band.top().reference(),
        }),
    }
}

/// The surface is zero feet above ground, whatever number accompanies it.
fn limit_feet(limit: AdvisoryAltitude) -> f64 {
    match limit.reference() {
        AdvisoryAltitudeReference::Surface => 0.0,
        _ => f64::from(limit.feet()),
    }
}

/// The query's altitude on the reference `limit` is stated on.
fn on_reference(limit: AdvisoryAltitude, altitude: QueryAltitude) -> Option<f64> {
    match limit.reference() {
        AdvisoryAltitudeReference::MeanSeaLevel => altitude.msl_ft,
        AdvisoryAltitudeReference::FlightLevel => altitude.pressure_altitude_ft,
        AdvisoryAltitudeReference::AboveGroundLevel | AdvisoryAltitudeReference::Surface => {
            altitude.agl_ft
        }
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests;
