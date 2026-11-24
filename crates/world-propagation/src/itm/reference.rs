//! The reference attenuation A_ref: line-of-sight, diffraction and
//! troposcatter lines joined at their transition distances (NTIA
//! `LongleyRice`).

use crate::error::PropagationError;
use crate::itm::complex::Complex;
use crate::itm::diffraction::{DiffractionContext, diffraction_loss};
use crate::itm::line_of_sight::{LineOfSightContext, line_of_sight_loss};
use crate::itm::terrain::{PathParameters, fdim};
use crate::itm::troposcatter::{CarriedH0, troposcatter_loss};
use crate::itm::{ItmWarnings, PropagationMode};

/// The troposcatter loss the reference code returns where the function
/// is undefined; any value at or above 1000 dB means "no troposcatter".
const UNDEFINED_SCATTER_DB: f64 = 1001.0;

/// A_ref (dB, not negative) and the propagation mode.
pub(crate) fn reference_attenuation(
    path: &PathParameters,
    h_m: [f64; 2],
    f_mhz: f64,
    z_g: Complex,
    gamma_e: f64,
    n_s: f64,
    warnings: &mut ItmWarnings,
) -> Result<(f64, PropagationMode), PropagationError> {
    let a_e = 1.0 / gamma_e;
    let d_hzn_s = [
        libm::sqrt(2.0 * path.h_e_m[0] * a_e),
        libm::sqrt(2.0 * path.h_e_m[1] * a_e),
    ];
    let d_sml = d_hzn_s[0] + d_hzn_s[1];
    let d_ml = path.d_hzn_m[0] + path.d_hzn_m[1];
    let theta_los = -(path.theta_hzn[0] + path.theta_hzn[1]).max(-d_ml / a_e);
    geometry_warnings(path, d_hzn_s, warnings);
    check_medium(n_s, a_e, z_g, warnings)?;

    let ctx = DiffractionContext {
        path: *path,
        h_m,
        z_g,
        a_e_m: a_e,
        theta_los,
        d_sml_m: d_sml,
        f_mhz,
    };
    let scale = libm::pow(a_e * a_e / f_mhz, 1.0 / 3.0);
    let d_3 = d_sml.max(d_ml + 5.0 * scale);
    let d_4 = d_3 + 10.0 * scale;
    let a_3 = diffraction_loss(&ctx, d_3);
    let a_4 = diffraction_loss(&ctx, d_4);
    let m_d = (a_4 - a_3) / (d_4 - d_3);
    let a_d0 = a_3 - m_d * d_3;
    distance_warnings(path, warnings);

    let (a_ref, mode) = if path.d_m < d_sml {
        let los = LineOfSightContext {
            h_e_m: path.h_e_m,
            z_g,
            delta_h_m: path.delta_h_m,
            m_d,
            a_d0_db: a_d0,
            d_sml_m: d_sml,
            f_mhz,
        };
        (
            line_of_sight_reference(&los, path.d_m, d_ml),
            PropagationMode::LineOfSight,
        )
    } else {
        let scatter = |d_m: f64, carried: &mut CarriedH0| {
            troposcatter_loss(path, d_m, a_e, n_s, f_mhz, theta_los, carried)
                .unwrap_or(UNDEFINED_SCATTER_DB)
        };
        let mut carried = CarriedH0::None;
        let d_5 = d_ml + 200.0e3;
        let d_6 = d_ml + 400.0e3;
        let a_6 = scatter(d_6, &mut carried);
        let a_5 = scatter(d_5, &mut carried);
        let (m_s, a_s0, d_x) = if a_5 < 1000.0 {
            let m_s = (a_6 - a_5) / 200.0e3;
            let d_x = d_sml
                .max(d_ml + 1.088 * scale * libm::log(f_mhz))
                .max((a_5 - a_d0 - m_s * d_5) / (m_d - m_s));
            (m_s, (m_d - m_s) * d_x + a_d0, d_x)
        } else {
            (m_d, a_d0, 10.0e6)
        };
        if path.d_m > d_x {
            (m_s * path.d_m + a_s0, PropagationMode::Troposcatter)
        } else {
            (m_d * path.d_m + a_d0, PropagationMode::Diffraction)
        }
    };
    Ok((a_ref.max(0.0), mode))
}

/// The line-of-sight line fitted through two line-of-sight losses and
/// the diffraction line at the smooth-Earth horizon distance
/// ([ERL 79-ITS 67 eqs 3.16 to 3.21]).
fn line_of_sight_reference(los: &LineOfSightContext, d_m: f64, d_ml: f64) -> f64 {
    let (m_d, a_d0, d_sml) = (los.m_d, los.a_d0_db, los.d_sml_m);
    let a_sml = d_sml * m_d + a_d0;
    let mut d_0 = 0.04 * los.f_mhz * los.h_e_m[0] * los.h_e_m[1];
    let d_1 = if a_d0 >= 0.0 {
        d_0 = d_0.min(0.5 * d_ml);
        d_0 + 0.25 * (d_ml - d_0)
    } else {
        (-a_d0 / m_d).max(0.25 * d_ml)
    };
    let a_1 = line_of_sight_loss(los, d_1);
    let mut flag = false;
    let mut k_1 = 0.0;
    let mut k_2 = 0.0;
    if d_0 < d_1 {
        let a_0 = line_of_sight_loss(los, d_0);
        let q = libm::log(d_sml / d_0);
        k_2 = (((d_sml - d_0) * (a_1 - a_0) - (d_1 - d_0) * (a_sml - a_0))
            / ((d_sml - d_0) * libm::log(d_1 / d_0) - (d_1 - d_0) * q))
            .max(0.0);
        flag = a_d0 > 0.0 || k_2 > 0.0;
        if flag {
            k_1 = (a_sml - a_0 - k_2 * q) / (d_sml - d_0);
            if k_1 < 0.0 {
                k_1 = 0.0;
                k_2 = fdim(a_sml, a_0) / q;
                if k_2 == 0.0 {
                    k_1 = m_d;
                }
            }
        }
    }
    if !flag {
        k_1 = fdim(a_sml, a_1) / (d_sml - d_1);
        k_2 = 0.0;
        if k_1 == 0.0 {
            k_1 = m_d;
        }
    }
    let a_o = a_sml - k_1 * d_sml - k_2 * libm::log(d_sml);
    a_o + k_1 * d_m + k_2 * libm::log(d_m)
}

/// Refuses a surface refractivity, effective Earth or ground impedance
/// outside what the model was fitted for.
fn check_medium(
    n_s: f64,
    a_e: f64,
    z_g: Complex,
    warnings: &mut ItmWarnings,
) -> Result<(), PropagationError> {
    if !(150.0..=400.0).contains(&n_s) {
        return Err(PropagationError::InvalidParameter {
            name: "surface_refractivity",
            value: n_s,
        });
    }
    if n_s < 250.0 {
        warnings.insert(ItmWarnings::SURFACE_REFRACTIVITY);
    }
    if !(4.0e6..=13_333_333.0).contains(&a_e) {
        return Err(PropagationError::InvalidParameter {
            name: "effective_earth_radius_m",
            value: a_e,
        });
    }
    if z_g.re <= libm::fabs(z_g.im) {
        return Err(PropagationError::InvalidParameter {
            name: "ground_impedance",
            value: z_g.re,
        });
    }
    Ok(())
}

/// Flags horizons the small-angle geometry may not describe.
fn geometry_warnings(path: &PathParameters, d_hzn_s: [f64; 2], warnings: &mut ItmWarnings) {
    let flags = [
        (
            ItmWarnings::TX_HORIZON_ANGLE,
            ItmWarnings::TX_HORIZON_DISTANCE_1,
            ItmWarnings::TX_HORIZON_DISTANCE_2,
        ),
        (
            ItmWarnings::RX_HORIZON_ANGLE,
            ItmWarnings::RX_HORIZON_DISTANCE_1,
            ItmWarnings::RX_HORIZON_DISTANCE_2,
        ),
    ];
    for (i, (angle, near, far)) in flags.into_iter().enumerate() {
        let theta = path.theta_hzn.get(i).copied().unwrap_or(f64::NAN);
        let d_hzn = path.d_hzn_m.get(i).copied().unwrap_or(f64::NAN);
        let d_s = d_hzn_s.get(i).copied().unwrap_or(f64::NAN);
        if libm::fabs(theta) > 200.0e-3 {
            warnings.insert(angle);
        }
        if d_hzn < 0.1 * d_s {
            warnings.insert(near);
        }
        if d_hzn > 3.0 * d_s {
            warnings.insert(far);
        }
    }
}

/// Flags path lengths near or beyond the model's range.
fn distance_warnings(path: &PathParameters, warnings: &mut ItmWarnings) {
    let d = path.d_m;
    if d < libm::fabs(path.h_e_m[0] - path.h_e_m[1]) / 200.0e-3 {
        warnings.insert(ItmWarnings::PATH_DISTANCE_TOO_SMALL_1);
    }
    if d < 1.0e3 {
        warnings.insert(ItmWarnings::PATH_DISTANCE_TOO_SMALL_2);
    }
    if d > 1000.0e3 {
        warnings.insert(ItmWarnings::PATH_DISTANCE_TOO_BIG_1);
    }
    if d > 2000.0e3 {
        warnings.insert(ItmWarnings::PATH_DISTANCE_TOO_BIG_2);
    }
}
