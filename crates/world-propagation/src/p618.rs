//! Recommendation ITU-R P.618-14, *Propagation data and prediction methods
//! required for the design of Earth-space telecommunication systems*.
//!
//! Computes total tropospheric slant-path attenuation (`1..=55 GHz`,
//! `5..=90°` elevation, `0.001..=50 %` exceedance time) and its constituent
//! components over caller-supplied, SHA-256-verified atmospheric inputs (no
//! ITU digital grids are bundled):
//! - Gaseous oxygen and water-vapour absorption (ITU-R P.676),
//! - Cloud liquid-water attenuation (ITU-R P.840-9),
//! - Rain attenuation (ITU-R P.618-14 §2.2.1.1, ITU-R P.838-3, ITU-R P.839-4),
//! - Tropospheric amplitude scintillation fading (ITU-R P.618-14 §2.4.1),
//! - Combined total attenuation (ITU-R P.618-14 §2.5), and
//! - Hydrometeor cross-polar discrimination (ITU-R P.618-14 §4.1).

mod cloud;
mod gaseous;
mod integrity;
mod rain;
mod scintillation;
mod site;

#[cfg(test)]
mod tests;

pub use integrity::{
    Sha256, VerifiedGridProvider, parse_sha256_hex, sha256, verify_provider_integrity,
};
pub use site::{AtmosphericGridProvider, SiteAtmosphere};

use crate::error::{PropagationError, positive};
use crate::loss::{Loss, PathLoss, Signal};
use crate::units::{Percentage, Polarisation};

/// Lowest frequency covered by the ITU-R P.618-14 total-attenuation method, Hz.
pub const MIN_FREQUENCY_HZ: f64 = 1.0e9;

/// Highest frequency covered by the ITU-R P.618-14 rain/total-attenuation method, Hz.
pub const MAX_FREQUENCY_HZ: f64 = 55.0e9;

/// Lowest slant-path elevation angle covered, degrees.
pub const MIN_ELEVATION_DEG: f64 = 5.0;

/// Highest slant-path elevation angle covered (zenith), degrees.
pub const MAX_ELEVATION_DEG: f64 = 90.0;

/// Lowest annual exceedance time percentage covered, %.
pub const MIN_PERCENTAGE: f64 = 0.001;

/// Highest annual exceedance time percentage covered, %.
pub const MAX_PERCENTAGE: f64 = 50.0;

/// Typical earth-station antenna aperture efficiency for scintillation averaging.
pub const DEFAULT_ANTENNA_EFFICIENCY: f64 = 0.6;

/// ITU-R P.618-14 Earth-space propagation model configured for a ground station.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P618 {
    site: SiteAtmosphere,
    station_latitude_deg: f64,
    station_altitude_km: f64,
    antenna_diameter_m: f64,
    antenna_efficiency: f64,
}

impl P618 {
    /// Creates a model for a ground station with pre-sampled `site` inputs.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] if any parameter or atmospheric
    /// input is non-finite or out of range (`|latitude| <= 90`, `diameter > 0`,
    /// `0 < efficiency <= 1`).
    pub fn new(
        site: SiteAtmosphere,
        station_latitude_deg: f64,
        station_altitude_km: f64,
        antenna_diameter_m: f64,
        antenna_efficiency: f64,
    ) -> Result<Self, PropagationError> {
        site.validate()?;
        check_station(
            station_latitude_deg,
            station_altitude_km,
            antenna_diameter_m,
            antenna_efficiency,
        )?;
        Ok(Self {
            site,
            station_latitude_deg,
            station_altitude_km,
            antenna_diameter_m,
            antenna_efficiency,
        })
    }

    /// Verifies `provider`'s SHA-256 digest and samples `site` at the ground
    /// station to build a [`P618`] model.
    ///
    /// # Errors
    ///
    /// [`PropagationError::GridIntegrityMismatch`] on digest failure;
    /// [`PropagationError::GridDataUnavailable`] or
    /// [`PropagationError::InvalidParameter`] on invalid station or grid data.
    pub fn from_verified_provider<P: AtmosphericGridProvider + ?Sized>(
        provider: &P,
        latitude_deg: f64,
        longitude_deg: f64,
        station_altitude_km: f64,
        antenna_diameter_m: f64,
        antenna_efficiency: f64,
    ) -> Result<Self, PropagationError> {
        let verified = VerifiedGridProvider::new(provider)?;
        check_lon(longitude_deg)?;
        let site =
            verified
                .provider()
                .sample_site(latitude_deg, longitude_deg, station_altitude_km)?;
        Self::new(
            site,
            latitude_deg,
            station_altitude_km,
            antenna_diameter_m,
            antenna_efficiency,
        )
    }

    /// Configured site atmospheric inputs.
    pub const fn site(&self) -> SiteAtmosphere {
        self.site
    }

    /// Station latitude, degrees.
    pub const fn station_latitude_deg(&self) -> f64 {
        self.station_latitude_deg
    }

    /// Station altitude above mean sea level, km.
    pub const fn station_altitude_km(&self) -> f64 {
        self.station_altitude_km
    }

    /// Ground-station antenna physical diameter, m.
    pub const fn antenna_diameter_m(&self) -> f64 {
        self.antenna_diameter_m
    }

    /// Ground-station antenna aperture efficiency in `(0, 1]`.
    pub const fn antenna_efficiency(&self) -> f64 {
        self.antenna_efficiency
    }
}

/// Slant-path geometry and outage exceedance probability for [`P618`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P618Path {
    /// Apparent Earth-station elevation angle, degrees (`5..=90`).
    pub elevation_deg: f64,
    /// Optional polarization tilt angle relative to local horizontal, degrees
    /// (`45.0` for circular). When `None`, derived from [`Signal::polarisation`]
    /// (`0°` horizontal, `90°` vertical).
    pub polarisation_tilt_deg: Option<f64>,
    /// Annual exceedance time percentage (`0.001..=50.0 %`).
    pub percentage: Percentage,
}

impl P618Path {
    /// Slant path with linear polarization tilt inferred from [`Signal::polarisation`].
    pub const fn new(elevation_deg: f64, percentage: Percentage) -> Self {
        Self {
            elevation_deg,
            polarisation_tilt_deg: None,
            percentage,
        }
    }

    /// Sets explicit polarization tilt relative to local horizontal, degrees
    /// (use `45.0` for circular polarization).
    pub const fn with_polarisation_tilt_deg(mut self, tilt_deg: f64) -> Self {
        self.polarisation_tilt_deg = Some(tilt_deg);
        self
    }
}

/// Station-resolved slant path for [`P618WithProvider`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P618StationPath {
    /// Ground station geodetic latitude, degrees (`-90..=90`).
    pub latitude_deg: f64,
    /// Ground station geodetic longitude, degrees (`-180..=360`).
    pub longitude_deg: f64,
    /// Ground station altitude above mean sea level, km.
    pub station_altitude_km: f64,
    /// Apparent elevation angle, degrees (`5..=90`).
    pub elevation_deg: f64,
    /// Optional polarization tilt angle relative to local horizontal, degrees.
    pub polarisation_tilt_deg: Option<f64>,
    /// Annual exceedance time percentage (`0.001..=50.0 %`).
    pub percentage: Percentage,
}

/// ITU-R P.618-14 evaluator backed by a SHA-256-verified [`AtmosphericGridProvider`].
#[derive(Clone, Copy, Debug)]
pub struct P618WithProvider<'a, P: ?Sized> {
    verified: VerifiedGridProvider<'a, P>,
    antenna_diameter_m: f64,
    antenna_efficiency: f64,
}

impl<'a, P: AtmosphericGridProvider + ?Sized> P618WithProvider<'a, P> {
    /// Verifies `provider`'s SHA-256 digest on construction and stores ground-station
    /// antenna parameters.
    ///
    /// # Errors
    ///
    /// [`PropagationError::GridIntegrityMismatch`] on SHA-256 mismatch, or
    /// [`PropagationError::InvalidParameter`] for invalid antenna geometry.
    pub fn new(
        provider: &'a P,
        antenna_diameter_m: f64,
        antenna_efficiency: f64,
    ) -> Result<Self, PropagationError> {
        let verified = VerifiedGridProvider::new(provider)?;
        positive("antenna_diameter_m", antenna_diameter_m)?;
        check_efficiency(antenna_efficiency)?;
        Ok(Self {
            verified,
            antenna_diameter_m,
            antenna_efficiency,
        })
    }

    /// SHA-256 digest verified for the underlying atmospheric grid provider.
    pub const fn verified_sha256(&self) -> [u8; 32] {
        self.verified.verified_sha256()
    }
}

/// ITU-R P.618-14 slant-path attenuation and component breakdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P618Loss {
    /// Total combined attenuation `A_T(p)` (P.618-14 §2.5), dB.
    pub total_db: f64,
    /// Total gaseous attenuation `A_G = A_o + A_w` (ITU-R P.676), dB.
    pub gaseous_db: f64,
    /// Dry-air / oxygen slant attenuation `A_o`, dB.
    pub gaseous_oxygen_db: f64,
    /// Water-vapour slant attenuation `A_w`, dB.
    pub gaseous_water_vapour_db: f64,
    /// Cloud liquid-water slant attenuation `A_C` (ITU-R P.840-9), dB.
    pub cloud_db: f64,
    /// Rain attenuation exceeded for `p %` of an average year `A_R(p)`, dB.
    pub rain_db: f64,
    /// Rain attenuation exceeded for `0.01 %` of an average year `A_0.01`, dB.
    pub rain_001_db: f64,
    /// Tropospheric amplitude scintillation fade depth `A_S(p)`, dB.
    pub scintillation_db: f64,
    /// Hydrometeor cross-polar discrimination `XPD(p)`, dB (`None` outside §4.1 domain).
    pub cross_polar_discrimination_db: Option<f64>,
    /// Effective slant rain path length `L_E`, km.
    pub effective_rain_path_km: f64,
    /// Specific rain attenuation `γ_R = k R_0.01^α`, dB/km.
    pub specific_rain_attenuation_db_km: f64,
}

impl Loss for P618Loss {
    fn loss_db(&self) -> f64 {
        self.total_db
    }
}

impl PathLoss<P618Path> for P618 {
    type Output = P618Loss;

    fn path_loss(
        &self,
        geometry: &P618Path,
        signal: &Signal,
    ) -> Result<P618Loss, PropagationError> {
        evaluate_p618(self, geometry, signal)
    }
}

impl<P: AtmosphericGridProvider + ?Sized> PathLoss<P618StationPath> for P618WithProvider<'_, P> {
    type Output = P618Loss;

    fn path_loss(
        &self,
        geometry: &P618StationPath,
        signal: &Signal,
    ) -> Result<P618Loss, PropagationError> {
        check_lon(geometry.longitude_deg)?;
        let site = self.verified.provider().sample_site(
            geometry.latitude_deg,
            geometry.longitude_deg,
            geometry.station_altitude_km,
        )?;
        let model = P618::new(
            site,
            geometry.latitude_deg,
            geometry.station_altitude_km,
            self.antenna_diameter_m,
            self.antenna_efficiency,
        )?;
        let path = P618Path {
            elevation_deg: geometry.elevation_deg,
            polarisation_tilt_deg: geometry.polarisation_tilt_deg,
            percentage: geometry.percentage,
        };
        evaluate_p618(&model, &path, signal)
    }
}

fn evaluate_p618(
    model: &P618,
    geometry: &P618Path,
    signal: &Signal,
) -> Result<P618Loss, PropagationError> {
    let f_hz = signal.frequency.hz();
    if !(MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ).contains(&f_hz) {
        return Err(PropagationError::FrequencyOutOfRange {
            frequency_hz: f_hz,
            min_hz: MIN_FREQUENCY_HZ,
            max_hz: MAX_FREQUENCY_HZ,
        });
    }
    let el_deg = check_elevation(geometry.elevation_deg)?;
    let p_pct = check_percentage(geometry.percentage.percent())?;
    let tau_deg = check_tilt(geometry.polarisation_tilt_deg, signal.polarisation)?;

    let f_ghz = signal.frequency.ghz();
    let site = model.site;

    let gas = gaseous::gaseous_attenuation(
        f_ghz,
        el_deg,
        site.surface_pressure_hpa,
        site.surface_temperature_k,
        site.water_vapour_density_g_m3,
    );
    let cloud_db = cloud::cloud_attenuation_db(f_ghz, el_deg, site.integrated_liquid_water_kg_m2);
    let rain_res = rain::rain_attenuation(rain::RainInputs {
        f_ghz,
        el_deg,
        tau_deg,
        lat_deg: model.station_latitude_deg,
        station_alt_km: model.station_altitude_km,
        h_0_km: site.zero_isotherm_height_km,
        r_001_mm_h: site.rain_rate_001_mm_h,
        p_percent: p_pct,
    });
    let scint_db = scintillation::scintillation_attenuation_db(
        f_ghz,
        el_deg,
        model.antenna_diameter_m,
        model.antenna_efficiency,
        site.wet_refractivity_n,
        p_pct,
    );
    let total_db = gas.total_db + libm::hypot(rain_res.rain_db + cloud_db, scint_db);
    let xpd_db =
        rain::cross_polar_discrimination_db(f_ghz, el_deg, tau_deg, p_pct, rain_res.rain_db);

    Ok(P618Loss {
        total_db,
        gaseous_db: gas.total_db,
        gaseous_oxygen_db: gas.oxygen_db,
        gaseous_water_vapour_db: gas.water_vapour_db,
        cloud_db,
        rain_db: rain_res.rain_db,
        rain_001_db: rain_res.rain_001_db,
        scintillation_db: scint_db,
        cross_polar_discrimination_db: xpd_db,
        effective_rain_path_km: rain_res.effective_path_length_km,
        specific_rain_attenuation_db_km: rain_res.specific_attenuation_db_km,
    })
}

fn check_station(
    latitude_deg: f64,
    altitude_km: f64,
    antenna_diameter_m: f64,
    antenna_efficiency: f64,
) -> Result<(), PropagationError> {
    if !latitude_deg.is_finite() || !(-90.0..=90.0).contains(&latitude_deg) {
        return Err(PropagationError::InvalidParameter {
            name: "station_latitude_deg",
            value: latitude_deg,
        });
    }
    if !altitude_km.is_finite() || altitude_km < -0.5 {
        return Err(PropagationError::InvalidParameter {
            name: "station_altitude_km",
            value: altitude_km,
        });
    }
    positive("antenna_diameter_m", antenna_diameter_m)?;
    check_efficiency(antenna_efficiency)?;
    Ok(())
}

fn check_lon(longitude_deg: f64) -> Result<(), PropagationError> {
    if longitude_deg.is_finite() && (-180.0..=360.0).contains(&longitude_deg) {
        Ok(())
    } else {
        Err(PropagationError::InvalidParameter {
            name: "longitude_deg",
            value: longitude_deg,
        })
    }
}

fn check_efficiency(eff: f64) -> Result<(), PropagationError> {
    if eff.is_finite() && eff > 0.0 && eff <= 1.0 {
        Ok(())
    } else {
        Err(PropagationError::InvalidParameter {
            name: "antenna_efficiency",
            value: eff,
        })
    }
}

fn check_elevation(elevation_deg: f64) -> Result<f64, PropagationError> {
    if elevation_deg.is_finite() && (MIN_ELEVATION_DEG..=MAX_ELEVATION_DEG).contains(&elevation_deg)
    {
        Ok(elevation_deg)
    } else {
        Err(PropagationError::InvalidParameter {
            name: "elevation_deg",
            value: elevation_deg,
        })
    }
}

fn check_percentage(percent: f64) -> Result<f64, PropagationError> {
    if percent.is_finite() && (MIN_PERCENTAGE..=MAX_PERCENTAGE).contains(&percent) {
        Ok(percent)
    } else {
        Err(PropagationError::InvalidParameter {
            name: "percentage",
            value: percent,
        })
    }
}

fn check_tilt(tilt_deg: Option<f64>, pol: Polarisation) -> Result<f64, PropagationError> {
    match tilt_deg {
        Some(t) => {
            if t.is_finite() && (-180.0..=180.0).contains(&t) {
                Ok(t)
            } else {
                Err(PropagationError::InvalidParameter {
                    name: "polarisation_tilt_deg",
                    value: t,
                })
            }
        }
        None => Ok(match pol {
            Polarisation::Horizontal => 0.0,
            Polarisation::Vertical => 90.0,
        }),
    }
}
