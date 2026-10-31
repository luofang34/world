//! Tier-0 zero-allocation physical Earth baseline (`Isa` + `WMM2025` + WGS-84 gravity).

use world_atmosphere::{
    Altitude, AtmosphereError, AtmosphereModel, AtmospherePoint, AtmosphereState, Isa,
};
use world_core::{DecimalYear, GeodeticPosition, normal_gravity};
use world_geomag::{GeomagError, MagneticField, wmm2025};

/// Allocation-free physical baseline of the Earth: standard [`Isa`] atmosphere,
/// [`wmm2025`] geomagnetic field and WGS-84 [`normal_gravity`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StandardWorld;

impl StandardWorld {
    /// Standard ISO 2533 atmosphere at `position`'s ellipsoidal height (treated as geometric MSL).
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError::OutsideDomain`] outside `-5 km..=80 km` geopotential.
    pub fn atmosphere_at(
        &self,
        position: GeodeticPosition,
        at: DecimalYear,
    ) -> Result<AtmosphereState, AtmosphereError> {
        Isa.state(AtmospherePoint {
            latitude_deg: position.latitude_deg,
            longitude_deg: position.longitude_deg,
            altitude: Altitude::GeometricMsl(position.ellipsoidal_height_m),
            at,
        })
    }

    /// NOAA WMM2025 geomagnetic field at `position` and `at`.
    ///
    /// # Errors
    ///
    /// Returns [`GeomagError`] if `position` is malformed or `at` lies outside WMM2025's window.
    pub fn magnetic_field_at(
        &self,
        position: GeodeticPosition,
        at: DecimalYear,
    ) -> Result<MagneticField, GeomagError> {
        wmm2025(position, at)
    }

    /// Somigliana normal gravity with free-air correction on `position`'s datum, m/s².
    ///
    /// # Errors
    ///
    /// Returns [`world_core::PositionError::NotWellFormed`] if `position` is malformed.
    pub fn gravity_m_s2(
        &self,
        position: GeodeticPosition,
    ) -> Result<f64, world_core::PositionError> {
        normal_gravity(position)
    }
}
