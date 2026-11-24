//! Time, location and situation variability about the reference
//! attenuation (NTIA `Variability`,
//! `InverseComplementaryCumulativeDistributionFunction`).

use crate::itm::diffraction::terrain_roughness;
use crate::itm::{Climate, VariabilityMode};

/// Effective Earth radius of the variability fit, m.
const A_9000_M: f64 = 9000.0e3;

/// Per-climate constants, columns in [`Climate`] order
/// ([TN101 Fig. 10.13; TN101v2 eqs III.69, III.70; Algorithm Table 5.1]).
const ALL_YEAR: [[f64; 7]; 5] = [
    [-9.67, -0.62, 1.26, -9.21, -0.62, -0.39, 3.15],
    [12.7, 9.19, 15.5, 9.05, 9.19, 2.86, 857.9],
    [
        144.9e3, 228.9e3, 262.6e3, 84.1e3, 228.9e3, 141.7e3, 2222.0e3,
    ],
    [
        190.3e3, 205.2e3, 185.2e3, 101.1e3, 205.2e3, 315.9e3, 164.8e3,
    ],
    [133.8e3, 143.6e3, 99.8e3, 98.6e3, 143.6e3, 167.4e3, 116.3e3],
];
const BSM1: [f64; 7] = [2.13, 2.66, 6.11, 1.98, 2.68, 6.86, 8.51];
const BSM2: [f64; 7] = [159.5, 7.67, 6.65, 13.11, 7.16, 10.38, 169.8];
const XSM1: [f64; 7] = [762.2e3, 100.4e3, 138.2e3, 139.1e3, 93.7e3, 187.8e3, 609.8e3];
const XSM2: [f64; 7] = [
    123.6e3, 172.5e3, 242.2e3, 132.7e3, 186.8e3, 169.6e3, 119.9e3,
];
const XSM3: [f64; 7] = [94.5e3, 136.4e3, 178.6e3, 193.5e3, 133.5e3, 108.9e3, 106.6e3];
const BSP1: [f64; 7] = [2.11, 6.87, 10.08, 3.68, 4.75, 8.58, 8.43];
const BSP2: [f64; 7] = [102.3, 15.53, 9.60, 159.3, 8.12, 13.97, 8.19];
const XSP1: [f64; 7] = [636.9e3, 138.7e3, 165.3e3, 464.4e3, 93.2e3, 216.0e3, 136.2e3];
const XSP2: [f64; 7] = [134.8e3, 143.7e3, 225.7e3, 93.1e3, 135.9e3, 152.0e3, 188.5e3];
const XSP3: [f64; 7] = [95.6e3, 98.6e3, 129.7e3, 94.2e3, 113.4e3, 122.7e3, 122.9e3];
const C_D: [f64; 7] = [1.224, 0.801, 1.380, 1.000, 1.224, 1.518, 1.518];
const Z_D: [f64; 7] = [1.282, 2.161, 1.282, 20.0, 1.282, 1.282, 1.282];
const BFM1: [f64; 7] = [1.0, 1.0, 1.0, 1.0, 0.92, 1.0, 1.0];
const BFM2: [f64; 7] = [0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0];
const BFM3: [f64; 7] = [0.0, 0.0, 0.0, 0.0, 1.77, 0.0, 0.0];
const BFP1: [f64; 7] = [1.0, 0.93, 1.0, 0.93, 0.93, 1.0, 1.0];
const BFP2: [f64; 7] = [0.0, 0.31, 0.0, 0.19, 0.31, 0.0, 0.0];
const BFP3: [f64; 7] = [0.0, 2.00, 0.0, 1.79, 2.00, 0.0, 0.0];

/// |z| beyond which a deviate is far in its tail.
const EXTREME_DEVIATE: f64 = 3.10;

/// The inputs the variability needs beyond the reference attenuation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VariabilityInputs {
    /// Time, location and situation as fractions in (0, 1).
    pub quantiles: [f64; 3],
    pub h_e_m: [f64; 2],
    pub delta_h_m: f64,
    pub f_mhz: f64,
    pub d_m: f64,
    pub climate: Climate,
    pub mode: VariabilityMode,
    pub eliminate_location: bool,
    pub eliminate_situation: bool,
}

/// The attenuation relative to free space not exceeded at the requested
/// quantiles, dB, and whether any deviate is beyond ±3.10.
pub(crate) fn variability(a_ref_db: f64, v: &VariabilityInputs) -> (f64, bool) {
    let climate = v.climate.index();
    let wn = v.f_mhz / 47.7;
    let d_e = effective_distance(v.h_e_m, v.d_m, wn);
    let sigma_s = if v.eliminate_situation {
        0.0
    } else {
        5.0 + 3.0 * libm::exp(-d_e / 100.0e3)
    };
    let year = |row: usize| ALL_YEAR.get(row).map_or(f64::NAN, |r| col(r, climate));
    let v_med = curve([year(0), year(1), year(2), year(3), year(4)], d_e);
    let ([z_t, z_l, z_s], extreme) = folded_deviates(v.quantiles, v.mode);
    let sigma_l = if v.eliminate_location {
        0.0
    } else {
        let delta_h_d = terrain_roughness(v.d_m, v.delta_h_m);
        10.0 * wn * delta_h_d / (wn * delta_h_d + 13.0)
    };
    let sigma_t = time_deviation(climate, d_e, wn, z_t);
    let (y_r, y_s) = combine_deviates(v.mode, [z_t, z_l, z_s], [sigma_t, sigma_l, sigma_s]);
    let result = a_ref_db - v_med - y_r - y_s;
    // Soft floor for predicted gains over free space [Algorithm eq. 52].
    let result = if result < 0.0 {
        result * (29.0 - result) / (29.0 - 10.0 * result)
    } else {
        result
    };
    (result, extreme)
}

/// Climate-indexed lookup into a 7-column constant table.
fn col(row: &[f64; 7], climate: usize) -> f64 {
    row.get(climate).copied().unwrap_or(f64::NAN)
}

/// Effective distance d_e (m) for the variability curves.
fn effective_distance(h_e_m: [f64; 2], d_m: f64, wn: f64) -> f64 {
    let d_ex = libm::sqrt(2.0 * A_9000_M * h_e_m[0])
        + libm::sqrt(2.0 * A_9000_M * h_e_m[1])
        + libm::pow(575.7e12 / wn, 1.0 / 3.0);
    if d_m < d_ex {
        130.0e3 * d_m / d_ex
    } else {
        130.0e3 + d_m - d_ex
    }
}

/// Standard normal deviates folded according to `mode`, plus tail check.
fn folded_deviates(quantiles: [f64; 3], mode: VariabilityMode) -> ([f64; 3], bool) {
    let [time, location, situation] = quantiles;
    let mut z_t = inverse_ccdf(time);
    let mut z_l = inverse_ccdf(location);
    let z_s = inverse_ccdf(situation);
    match mode {
        VariabilityMode::SingleMessage => {
            z_t = z_s;
            z_l = z_s;
        }
        VariabilityMode::Accidental => z_l = z_s,
        VariabilityMode::Mobile => z_l = z_t,
        VariabilityMode::Broadcast => {}
    }
    let z = [z_t, z_l, z_s];
    let extreme = z.iter().any(|val| libm::fabs(*val) > EXTREME_DEVIATE);
    (z, extreme)
}

/// Time variability standard deviation σ_t (dB).
fn time_deviation(climate: usize, d_e: f64, wn: f64, z_t: f64) -> f64 {
    let q = libm::log(0.133 * wn);
    let g = |b1: f64, b2: f64, b3: f64| b1 + b2 / ((b3 * q) * (b3 * q) + 1.0);
    let g_minus = g(
        col(&BFM1, climate),
        col(&BFM2, climate),
        col(&BFM3, climate),
    );
    let g_plus = g(
        col(&BFP1, climate),
        col(&BFP2, climate),
        col(&BFP3, climate),
    );
    let sigma_t_minus = curve(
        [
            col(&BSM1, climate),
            col(&BSM2, climate),
            col(&XSM1, climate),
            col(&XSM2, climate),
            col(&XSM3, climate),
        ],
        d_e,
    ) * g_minus;
    let sigma_t_plus = curve(
        [
            col(&BSP1, climate),
            col(&BSP2, climate),
            col(&XSP1, climate),
            col(&XSP2, climate),
            col(&XSP3, climate),
        ],
        d_e,
    ) * g_plus;
    let sigma_td = col(&C_D, climate) * sigma_t_plus;
    let z_d = col(&Z_D, climate);
    let tgtd = (sigma_t_plus - sigma_td) * z_d;
    if z_t < 0.0 {
        sigma_t_minus
    } else if z_t <= z_d {
        sigma_t_plus
    } else {
        sigma_td + tgtd / z_t
    }
}

/// Combines time, location and situation deviations (Y_r, Y_s) for `mode`.
fn combine_deviates(
    mode: VariabilityMode,
    [z_t, z_l, z_s]: [f64; 3],
    [sigma_t, sigma_l, sigma_s]: [f64; 3],
) -> (f64, f64) {
    let y_l = sigma_l * z_l;
    let y_t = sigma_t * z_t;
    let z_s2 = z_s * z_s;
    let y_s_temp = sigma_s * sigma_s + y_t * y_t / (7.8 + z_s2) + y_l * y_l / (24.0 + z_s2);
    match mode {
        VariabilityMode::SingleMessage => (
            0.0,
            libm::sqrt(sigma_t * sigma_t + sigma_l * sigma_l + y_s_temp) * z_s,
        ),
        VariabilityMode::Accidental => (y_t, libm::sqrt(sigma_l * sigma_l + y_s_temp) * z_s),
        VariabilityMode::Mobile => (
            libm::sqrt(sigma_t * sigma_t + sigma_l * sigma_l) * z_t,
            libm::sqrt(y_s_temp) * z_s,
        ),
        VariabilityMode::Broadcast => (y_t + y_l, libm::sqrt(y_s_temp) * z_s),
    }
}

/// The climate curves of [TN101v2 eqs III.69, III.70] at effective
/// distance `d_e_m`: c = [c1, c2, x1, x2, x3].
fn curve(c: [f64; 5], d_e_m: f64) -> f64 {
    let [c1, c2, x1, x2, x3] = c;
    let r = (d_e_m - x2) / x3;
    let s = (d_e_m / x1) * (d_e_m / x1);
    (c1 + c2 / (1.0 + r * r)) * s / (1.0 + s)
}

/// Q⁻¹(q), the inverse of the complementary standard normal distribution
/// by the Hastings rational approximation (error below 4.5e-4).
pub(crate) fn inverse_ccdf(q: f64) -> f64 {
    const C_0: f64 = 2.515516;
    const C_1: f64 = 0.802853;
    const C_2: f64 = 0.010328;
    const D_1: f64 = 1.432788;
    const D_2: f64 = 0.189269;
    const D_3: f64 = 0.001308;
    let x = if q > 0.5 { 1.0 - q } else { q };
    let t = libm::sqrt(-2.0 * libm::log(x));
    let zeta = ((C_2 * t + C_1) * t + C_0) / (((D_3 * t + D_2) * t + D_1) * t + 1.0);
    if q > 0.5 { -(t - zeta) } else { t - zeta }
}
