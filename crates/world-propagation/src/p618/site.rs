//! Ground-station atmospheric inputs and SHA-256-verifiable grid provider trait.

use world_core::Provenance;

use crate::error::{PropagationError, finite, non_negative, positive};

/// Site meteorological and rain-climatic parameters at a ground station.
///
/// Supplied either directly by the caller or sampled from an
/// [`AtmosphericGridProvider`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteAtmosphere {
    /// Point rainfall rate exceeded for 0.01 % of an average year, mm/h (ITU-R P.837).
    pub rain_rate_001_mm_h: f64,
    /// Annual mean 0 °C isotherm height above mean sea level, km (ITU-R P.839).
    pub zero_isotherm_height_km: f64,
    /// Surface air temperature, K (ITU-R P.835 / P.676).
    pub surface_temperature_k: f64,
    /// Total surface barometric pressure, hPa (ITU-R P.835 / P.676).
    pub surface_pressure_hpa: f64,
    /// Surface water-vapour density, g/m³ (ITU-R P.835 / P.676).
    pub water_vapour_density_g_m3: f64,
    /// Reduced total columnar cloud liquid water content (`273.15 K`), kg/m² (ITU-R P.840).
    pub integrated_liquid_water_kg_m2: f64,
    /// Median surface wet radio refractivity `N_wet`, N-units (ITU-R P.453 / P.618).
    pub wet_refractivity_n: f64,
}

impl SiteAtmosphere {
    /// Builds site inputs using ITU-R P.835 sea-level reference dry/moist air
    /// (`288.15 K`, `1013.25 hPa`, `7.5 g/m³`) together with caller-provided
    /// rain, isotherm height, cloud liquid water, and `N_wet`.
    pub const fn itu_reference(
        rain_rate_001_mm_h: f64,
        zero_isotherm_height_km: f64,
        integrated_liquid_water_kg_m2: f64,
        wet_refractivity_n: f64,
    ) -> Self {
        Self {
            rain_rate_001_mm_h,
            zero_isotherm_height_km,
            surface_temperature_k: 288.15,
            surface_pressure_hpa: 1013.25,
            water_vapour_density_g_m3: 7.5,
            integrated_liquid_water_kg_m2,
            wet_refractivity_n,
        }
    }

    /// Validates all meteorological values for finiteness and physical signs.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] when any field is non-finite or
    /// outside physical bounds.
    pub fn validate(&self) -> Result<(), PropagationError> {
        non_negative("rain_rate_001_mm_h", self.rain_rate_001_mm_h)?;
        finite("zero_isotherm_height_km", self.zero_isotherm_height_km)?;
        positive("surface_temperature_k", self.surface_temperature_k)?;
        positive("surface_pressure_hpa", self.surface_pressure_hpa)?;
        non_negative("water_vapour_density_g_m3", self.water_vapour_density_g_m3)?;
        non_negative(
            "integrated_liquid_water_kg_m2",
            self.integrated_liquid_water_kg_m2,
        )?;
        non_negative("wet_refractivity_n", self.wet_refractivity_n)?;
        Ok(())
    }
}

/// Caller-supplied provider of ITU atmospheric digital products (P.837, P.839,
/// P.835/P.836, P.840, P.453) with SHA-256 content-integrity contract.
///
/// ITU data tables are never bundled in `world-propagation`; callers load
/// external grids at runtime and expose their canonical bytes and expected
/// SHA-256 digest here so [`super::verify_provider_integrity`] /
/// [`super::VerifiedGridProvider`] can verify dataset integrity before sampling.
pub trait AtmosphericGridProvider {
    /// Optional dataset provenance metadata (including `source_sha256` hex).
    fn provenance(&self) -> Option<Provenance> {
        None
    }

    /// Expected 32-byte SHA-256 digest of the canonical grid content.
    fn expected_sha256(&self) -> Option<[u8; 32]>;

    /// Feeds the canonical grid bytes sequentially to `visitor` for hashing.
    fn visit_grid_bytes(&self, visitor: &mut dyn FnMut(&[u8]));

    /// Samples meteorological and climatic inputs at `(latitude_deg, longitude_deg)`
    /// and ground-station elevation `station_altitude_km` above mean sea level.
    ///
    /// # Errors
    ///
    /// [`PropagationError::GridDataUnavailable`] outside provider coverage, or
    /// [`PropagationError::InvalidParameter`] for invalid station coordinates.
    fn sample_site(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
        station_altitude_km: f64,
    ) -> Result<SiteAtmosphere, PropagationError>;
}
