//! The complex Fresnel integral by the Boersma approximation the
//! Recommendation gives (P.526-16 §2.7, eqs 6 to 10).

use core::f64::consts::{FRAC_PI_2, SQRT_2};

/// Boersma coefficients a_n and b_n of eq. (8a), for 0 <= x < 4.
const A: [f64; 12] = [
    1.595_769_140,
    -0.000_001_702,
    -6.808_568_854,
    -0.000_576_361,
    6.920_691_902,
    -0.016_898_657,
    -3.050_485_660,
    -0.075_752_419,
    0.850_663_781,
    -0.025_639_041,
    -0.150_230_960,
    0.034_404_779,
];
const B: [f64; 12] = [
    -0.000_000_033,
    4.255_387_524,
    -0.000_092_810,
    -7.780_020_400,
    -0.009_520_895,
    5.075_161_298,
    -0.138_341_947,
    -1.363_729_124,
    -0.403_349_276,
    0.702_222_016,
    -0.216_195_929,
    0.019_547_031,
];
/// Boersma coefficients c_n and d_n of eq. (8b), for x >= 4.
const C: [f64; 12] = [
    0.000_000_000,
    -0.024_933_975,
    0.000_003_936,
    0.005_770_956,
    0.000_689_892,
    -0.009_497_136,
    0.011_948_809,
    -0.006_748_873,
    0.000_246_420,
    0.002_102_967,
    -0.001_217_930,
    0.000_233_939,
];
const D: [f64; 12] = [
    0.199_471_140,
    0.000_000_023,
    -0.009_351_341,
    0.000_023_006,
    0.004_851_466,
    0.001_903_218,
    -0.017_122_914,
    0.029_064_067,
    -0.027_928_955,
    0.016_497_308,
    -0.005_598_515,
    0.000_838_386,
];

/// The x of eq. (9) that separates the two expansions.
const SERIES_SPLIT: f64 = 4.0;

/// Σ coefficients[n] tⁿ by Horner's rule.
fn polynomial(coefficients: &[f64; 12], t: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |sum, c| sum * t + c)
}

/// exp(jx) √t Σ (pₙ − j qₙ) tⁿ, the common shape of eqs (8a) and (8b),
/// as (real, imaginary).
fn rotated_series(p: &[f64; 12], q: &[f64; 12], t: f64, x: f64) -> (f64, f64) {
    let sum_p = polynomial(p, t);
    let sum_q = polynomial(q, t);
    let root = libm::sqrt(t);
    let (sin_x, cos_x) = (libm::sin(x), libm::cos(x));
    (
        root * (sum_p * cos_x + sum_q * sin_x),
        root * (sum_p * sin_x - sum_q * cos_x),
    )
}

/// The Fresnel cosine and sine integrals (C(ν), S(ν)) of eqs (7a), (7b),
/// for any finite ν (eq. 10 gives negative ν).
pub fn fresnel_integrals(nu: f64) -> (f64, f64) {
    let magnitude = libm::fabs(nu);
    let x = FRAC_PI_2 * magnitude * magnitude;
    let (c, s) = if x < SERIES_SPLIT {
        rotated_series(&A, &B, x / 4.0, x)
    } else {
        let (re, im) = rotated_series(&C, &D, 4.0 / x, x);
        (0.5 + re, 0.5 + im)
    };
    if nu < 0.0 { (-c, -s) } else { (c, s) }
}

/// |(1 + j)/2 − F_c(ν)| / √2: the diffracted field relative to free space
/// behind a knife edge, the argument of the logarithm in eq. (30).
///
/// For x ≥ 4 the eq. (8b) series *is* the remainder, so deep in the
/// shadow the field is taken from it directly rather than as the small
/// difference of two numbers close to ½.
pub(crate) fn knife_edge_field(nu: f64) -> f64 {
    let magnitude = libm::fabs(nu);
    let x = FRAC_PI_2 * magnitude * magnitude;
    let (re, im) = if nu >= 0.0 && x >= SERIES_SPLIT {
        rotated_series(&C, &D, 4.0 / x, x)
    } else {
        let (c, s) = fresnel_integrals(nu);
        (0.5 - c, 0.5 - s)
    };
    libm::hypot(re, im) / SQRT_2
}
