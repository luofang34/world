//! Diffraction over a general terrestrial path (P.526-16 §4.5): the
//! Bullington construction (§4.5.1) and the complete method that combines
//! it with the smooth-Earth loss (§4.5.2, the "delta-Bullington" method).

use crate::error::{PropagationError, non_negative, positive};
use crate::loss::{Loss, PathLoss, Signal};
use crate::p526::knife_edge::knife_edge_loss_approx_db;
use crate::p526::smooth_earth::{
    Ground, MIN_FREQUENCY_HZ, TerminalPath, spherical_earth_loss, validate_ground,
};
use crate::profile::{Flattened, TerrainProfile, samples_km, validate};

/// A terminal-to-terminal path over a terrain profile.
#[derive(Clone, Copy, Debug)]
pub struct ProfilePath<'a, P: ?Sized> {
    /// The terrain, first and last samples under the antennas.
    pub profile: &'a P,
    /// Transmitting antenna height above the first sample's ground, m.
    pub tx_antenna_height_m: f64,
    /// Receiving antenna height above the last sample's ground, m.
    pub rx_antenna_height_m: f64,
}

/// The complete method of §4.5.2 for any path, line of sight or
/// trans-horizon, rough or smooth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaBullington {
    effective_earth_radius_m: f64,
    ground: Ground,
}

impl DeltaBullington {
    /// The method on an Earth of effective radius `effective_earth_radius_m`
    /// whose smooth-Earth term uses `ground`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] for a non-positive radius or
    /// permittivity, or a negative conductivity.
    pub fn new(effective_earth_radius_m: f64, ground: Ground) -> Result<Self, PropagationError> {
        positive("effective_earth_radius_m", effective_earth_radius_m)?;
        validate_ground(ground)?;
        Ok(Self {
            effective_earth_radius_m,
            ground,
        })
    }
}

/// The complete method's loss with the terms eq. (66) combines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaBullingtonLoss {
    /// L of eq. (66), dB.
    pub loss_db: f64,
    /// Bullington loss of the actual profile, L_ba, dB.
    pub actual_bullington_db: f64,
    /// Bullington loss of the smooth profile, L_bs, dB.
    pub smooth_bullington_db: f64,
    /// Smooth-Earth loss of §3.2 for the smooth profile, L_sph, dB.
    pub spherical_earth_db: f64,
    /// Height of the fitted smooth surface under the transmitter, h_st,
    /// m above sea level (eq. 63).
    pub smooth_surface_tx_m: f64,
    /// Height of the fitted smooth surface under the receiver, h_sr, m
    /// above sea level.
    pub smooth_surface_rx_m: f64,
}

impl Loss for DeltaBullingtonLoss {
    fn loss_db(&self) -> f64 {
        self.loss_db
    }
}

impl<P: TerrainProfile + ?Sized> PathLoss<ProfilePath<'_, P>> for DeltaBullington {
    type Output = DeltaBullingtonLoss;

    fn path_loss(
        &self,
        geometry: &ProfilePath<'_, P>,
        signal: &Signal,
    ) -> Result<DeltaBullingtonLoss, PropagationError> {
        let profile = geometry.profile;
        let ends = validate(profile, 3)?;
        let hts = ends.first_height_m
            + non_negative("tx_antenna_height_m", geometry.tx_antenna_height_m)?;
        let hrs =
            ends.last_height_m + non_negative("rx_antenna_height_m", geometry.rx_antenna_height_m)?;
        let ae = self.effective_earth_radius_m;
        let lambda = signal.frequency.wavelength_m();
        if signal.frequency.hz() < MIN_FREQUENCY_HZ {
            return Err(PropagationError::FrequencyOutOfRange {
                frequency_hz: signal.frequency.hz(),
                min_hz: MIN_FREQUENCY_HZ,
                max_hz: f64::INFINITY,
            });
        }
        let actual = bullington(profile, ends.length_km, hts, hrs, ae, lambda);
        let (hst, hsr) = smooth_surface(profile, ends.length_km, hts, hrs);
        let (hst, hsr) = (hst.min(ends.first_height_m), hsr.min(ends.last_height_m)); // (63)
        let (hts_smooth, hrs_smooth) = (hts - hst, hrs - hsr); // (64)
        let smooth = bullington(
            &Flattened(profile),
            ends.length_km,
            hts_smooth,
            hrs_smooth,
            ae,
            lambda,
        );
        let path = TerminalPath {
            distance_m: ends.length_km * 1.0e3,
            tx_height_m: hts_smooth,
            rx_height_m: hrs_smooth,
        };
        let spherical = spherical_earth_loss(&path, ae, self.ground, signal)?.loss_db;
        Ok(DeltaBullingtonLoss {
            loss_db: actual + (spherical - smooth).max(0.0), // (66)
            actual_bullington_db: actual,
            smooth_bullington_db: smooth,
            spherical_earth_db: spherical,
            smooth_surface_tx_m: hst,
            smooth_surface_rx_m: hsr,
        })
    }
}

/// The Bullington loss L_b of eq. (57), dB, for terminals `hts` and `hrs`
/// m above sea level over a profile `d_km` long.
///
/// Slopes are m/km against the sea-level chord, heights are lifted by the
/// Earth bulge 500 Cₑ dᵢ (d − dᵢ), and J is eq. (31).
fn bullington<P: TerrainProfile + ?Sized>(
    profile: &P,
    d_km: f64,
    hts: f64,
    hrs: f64,
    ae_m: f64,
    lambda_m: f64,
) -> f64 {
    let curvature = 1.0e3 / ae_m; // Cₑ, 1/km
    let bulged = |(di, hi): (f64, f64)| (di, hi + 500.0 * curvature * di * (d_km - di));
    let intermediate = || {
        samples_km(profile)
            .skip(1)
            .take(profile.len().saturating_sub(2))
            .map(bulged)
    };
    let max_of = |values: &mut dyn Iterator<Item = f64>| values.fold(f64::NEG_INFINITY, f64::max);
    let nu_scale = |di: f64| libm::sqrt(0.002 * d_km / (lambda_m * di * (d_km - di)));

    let s_tim = max_of(&mut intermediate().map(|(di, hi)| (hi - hts) / di)); // (49)
    let s_tr = (hrs - hts) / d_km; // (50)
    let nu = if s_tim < s_tr {
        max_of(
            &mut intermediate()
                .map(|(di, hi)| (hi - (hts * (d_km - di) + hrs * di) / d_km) * nu_scale(di)),
        ) // (51)
    } else {
        let s_rim = max_of(&mut intermediate().map(|(di, hi)| (hi - hrs) / (d_km - di))); // (53)
        let db = (hrs - hts + s_rim * d_km) / (s_tim + s_rim); // (54)
        (hts + s_tim * db - (hts * (d_km - db) + hrs * db) / d_km) * nu_scale(db) // (55)
    };
    let l_uc = knife_edge_loss_approx_db(nu); // (52), (56)
    l_uc + (1.0 - libm::exp(-l_uc / 6.0)) * (10.0 + 0.02 * d_km) // (57)
}

/// Provisional smooth-surface heights (h_stp, h_srp) of eqs (58) to (62),
/// m above sea level.
fn smooth_surface<P: TerrainProfile + ?Sized>(
    profile: &P,
    d_km: f64,
    hts: f64,
    hrs: f64,
) -> (f64, f64) {
    let mut v1 = 0.0;
    let mut v2 = 0.0;
    let mut previous: Option<(f64, f64)> = None;
    for (di, hi) in samples_km(profile) {
        if let Some((dp, hp)) = previous {
            v1 += (di - dp) * (hi + hp); // (58)
            v2 += (di - dp) * (hi * (2.0 * di + dp) + hp * (di + 2.0 * dp)); // (59)
        }
        previous = Some((di, hi));
    }
    let hstip = (2.0 * v1 * d_km - v2) / (d_km * d_km); // (60a)
    let hsrip = (v2 - v1 * d_km) / (d_km * d_km); // (60b)

    let mut h_obs = f64::NEG_INFINITY;
    let mut alpha_obt = f64::NEG_INFINITY;
    let mut alpha_obr = f64::NEG_INFINITY;
    for (di, hi) in samples_km(profile)
        .skip(1)
        .take(profile.len().saturating_sub(2))
    {
        let h_ob = hi - (hts * (d_km - di) + hrs * di) / d_km; // (61d)
        h_obs = h_obs.max(h_ob); // (61a)
        alpha_obt = alpha_obt.max(h_ob / di); // (61b)
        alpha_obr = alpha_obr.max(h_ob / (d_km - di)); // (61c)
    }
    if h_obs <= 0.0 {
        (hstip, hsrip) // (62a), (62b)
    } else {
        let g_t = alpha_obt / (alpha_obt + alpha_obr); // (62e)
        let g_r = alpha_obr / (alpha_obt + alpha_obr); // (62f)
        (hstip - h_obs * g_t, hsrip - h_obs * g_r) // (62c), (62d)
    }
}
