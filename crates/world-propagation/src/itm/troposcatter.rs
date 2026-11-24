//! Forward-scatter attenuation far beyond the horizon (NTIA
//! `TroposcatterLoss`, `H0Function`).

use core::f64::consts::SQRT_2;

use crate::itm::terrain::PathParameters;

/// The frequency-gain function H₀ carried from the far distance to the
/// near one: the Algorithm evaluates d₆ first and, where H₀ would exceed
/// 15 dB at d₅, reuses the d₆ value.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CarriedH0 {
    /// No distance evaluated yet.
    None,
    /// H₀ at the previous distance, dB.
    Value(f64),
}

/// A_scat at `d_m`, or `None` where both r₁ and r₂ are below 0.2 and the
/// function is undefined ([Algorithm eq. 4.63; TN101 §9]).
pub(crate) fn troposcatter_loss(
    path: &PathParameters,
    d_m: f64,
    a_e_m: f64,
    n_s: f64,
    f_mhz: f64,
    theta_los: f64,
    carried: &mut CarriedH0,
) -> Option<f64> {
    let wn = f_mhz / 47.7;
    let h_0 = match *carried {
        CarriedH0::Value(h) if h > 15.0 => h,
        _ => {
            let h = frequency_gain(path, d_m, a_e_m, n_s, wn)?;
            match *carried {
                CarriedH0::Value(previous) if h > 15.0 && previous >= 0.0 => previous,
                _ => h,
            }
        }
    };
    *carried = CarriedH0::Value(h_0);
    let th = d_m / a_e_m - theta_los;
    const D_0_M: f64 = 40.0e3;
    const H_M: f64 = 47.7;
    Some(
        f_function(th * d_m) + 10.0 * libm::log10(wn * H_M * th * th * th * th)
            - 0.1 * (n_s - 301.0) * libm::exp(-th * d_m / D_0_M)
            + h_0,
    )
}

/// H₀ before carrying ([TN101 eqs 9.3 to 9.5]).
fn frequency_gain(path: &PathParameters, d_m: f64, a_e_m: f64, n_s: f64, wn: f64) -> Option<f64> {
    let [h_e_tx, h_e_rx] = path.h_e_m;
    let mut ad = path.d_hzn_m[0] - path.d_hzn_m[1];
    let mut rr = h_e_rx / h_e_tx;
    if ad < 0.0 {
        ad = -ad;
        rr = 1.0 / rr;
    }
    let theta = path.theta_hzn[0] + path.theta_hzn[1] + d_m / a_e_m;
    let r_1 = 2.0 * wn * theta * h_e_tx;
    let r_2 = 2.0 * wn * theta * h_e_rx;
    if r_1 < 0.2 && r_2 < 0.2 {
        return None;
    }
    let s = (d_m - ad) / (d_m + ad);
    let q = (rr / s).max(0.1).min(10.0);
    let s = s.max(0.1);
    let h_0_m = (d_m - ad) * (d_m + ad) * theta * 0.25 / d_m;
    const Z_0_M: f64 = 1.7556e3;
    const Z_1_M: f64 = 8.0e3;
    let eta_s = (h_0_m / Z_0_M)
        * (1.0
            + (0.031 - n_s * 2.32e-3 + n_s * n_s * 5.67e-6)
                * libm::exp(-libm::pow((h_0_m / Z_1_M).min(1.7), 6.0)));
    let h_00 = (h0_function(r_1, eta_s) + h0_function(r_2, eta_s)) / 2.0;
    let delta =
        h_00.min(6.0 * (0.6 - libm::log10(eta_s.max(1.0))) * libm::log10(s) * libm::log10(q));
    let mut h_0 = (h_00 + delta).max(0.0);
    if eta_s < 1.0 {
        let special = 10.0
            * libm::log10(
                libm::pow((1.0 + SQRT_2 / r_1) * (1.0 + SQRT_2 / r_2), 2.0) * (r_1 + r_2)
                    / (r_1 + r_2 + 2.0 * SQRT_2),
            );
        h_0 = eta_s * h_0 + (1.0 - eta_s) * special;
    }
    Some(h_0)
}

/// The attenuation function F(θd) ([Algorithm 6.9]).
fn f_function(td: f64) -> f64 {
    let (a, b, c) = if td <= 10.0e3 {
        (133.4, 0.332e-3, -10.0)
    } else if td <= 70.0e3 {
        (104.6, 0.212e-3, -2.5)
    } else {
        (71.8, 0.157e-3, 5.0)
    };
    a + b * td + c * libm::log10(td)
}

/// H₀(r, η_s), interpolated between the curves of [Algorithm 6.13].
fn h0_function(r: f64, eta_s: f64) -> f64 {
    let eta_s = eta_s.max(1.0).min(5.0);
    let i = eta_s as usize;
    let q = eta_s - i as f64;
    let curve = |j: usize| {
        const A: [f64; 5] = [25.0, 80.0, 177.0, 395.0, 705.0];
        const B: [f64; 5] = [24.0, 45.0, 68.0, 80.0, 105.0];
        let a = A.get(j).copied().unwrap_or(f64::NAN);
        let b = B.get(j).copied().unwrap_or(f64::NAN);
        10.0 * libm::log10(1.0 + a * libm::pow(1.0 / r, 4.0) + b * (1.0 / r) * (1.0 / r))
    };
    let result = curve(i - 1);
    if q != 0.0 {
        (1.0 - q) * result + q * curve(i)
    } else {
        result
    }
}
