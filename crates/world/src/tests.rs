#[cfg(all(feature = "atmosphere", feature = "geomag"))]
#[test]
fn standard_world_evaluates_atmosphere_geomag_and_gravity() {
    use crate::{DecimalYear, GeodeticPosition, StandardWorld};

    let world = StandardWorld;
    let pos = GeodeticPosition::wgs84(39.18, -75.53, 900.0);
    let at = DecimalYear(2026.5);

    let air = world.atmosphere_at(pos, at).unwrap();
    assert!(air.density_kg_m3 > 1.0 && air.density_kg_m3 < 1.25);

    let mag = world.magnetic_field_at(pos, at).unwrap();
    assert!(mag.declination_rad().is_finite());

    let g = world.gravity_m_s2(pos).unwrap();
    assert!((g - 9.8).abs() < 0.05);
}

/// Compiles only while `$umbrella` and `$library` are one type: a function
/// pointer from one to the other cannot be the identity function otherwise.
#[cfg(feature = "moving")]
macro_rules! assert_same_type {
    ($($umbrella:ty = $library:ty;)+) => {
        $(let _: fn($library) -> $umbrella = ::core::convert::identity;)+
    };
}

/// The umbrella's traffic and weather vocabulary is the libraries' own. A
/// type declared in this workspace under one of these names would be a
/// second, shadow definition and would fail to compile here.
#[cfg(feature = "moving")]
#[test]
fn the_umbrella_re_exports_the_domain_libraries_own_types() {
    assert_same_type! {
        crate::surveillance::SurveillanceEngine = surveillance_core::SurveillanceEngine;
        crate::surveillance::TrafficObservation = surveillance_core::TrafficObservation;
        crate::surveillance::TrackSnapshot = surveillance_core::TrackSnapshot;
        crate::surveillance::TrackDelta = surveillance_core::TrackDelta;
        crate::surveillance::TrackKey = surveillance_core::TrackKey;
        crate::surveillance::AddressNamespace = surveillance_core::AddressNamespace;
        crate::surveillance::ProjectionBounds = surveillance_core::ProjectionBounds;
        crate::surveillance::DeliveryPath = surveillance_core::DeliveryPath;
        crate::surveillance::SourceRef = surveillance_core::SourceRef;
        crate::airmass::WeatherStore = airmass_core::WeatherStore;
        crate::airmass::WeatherIngress = airmass_core::WeatherIngress;
        crate::airmass::WeatherSnapshotHandle = airmass_core::WeatherSnapshotHandle;
        crate::airmass::FlightCategory = airmass_core::FlightCategory;
        crate::airmass::DeliveryPath = airmass_core::DeliveryPath;
        crate::airmass::SourceRef = airmass_core::SourceRef;
        crate::airmass::ProductProvenance = airmass_core::ProductProvenance;
    }

    // The world's members are those types, not wrappers around them.
    fn members(
        world: crate::MovingWorld,
    ) -> (
        surveillance_core::SurveillanceEngine,
        airmass_core::WeatherStore,
    ) {
        (world.surveillance, world.weather)
    }
    let _ = members;
}
