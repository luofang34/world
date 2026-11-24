//! Regularized incomplete gamma and Nakagami-m intensity quantile for
//! ITU-R P.531 amplitude scintillation.

use core::f64::consts::SQRT_2;

/// Cutoff above which Gamma(m, m) is evaluated via Wilson–Hilferty normal
/// approximation rather than series/continued-fraction summation.
const LARGE_M_CUTOFF: f64 = 1_000.0;

/// Maximum series / continued-fraction iterations.
const MAX_GAMMA_ITERS: usize = 500;

/// Fade depth (dB >= 0) exceeded for `p_fraction` in (0, 1) under a
/// unit-mean Nakagami-m intensity distribution with shape `m = 1 / S4^2`.
pub(crate) fn nakagami_fade_db(s4: f64, p_fraction: f64) -> f64 {
    if s4 <= 0.0 {
        return 0.0;
    }
    let m = 1.0 / (s4 * s4);
    let intensity = if m > LARGE_M_CUTOFF {
        wilson_hilferty_intensity(m, p_fraction)
    } else {
        invert_gamma_cdf(m, p_fraction)
    };
    if intensity <= 0.0 {
        return 0.0;
    }
    (-10.0 * libm::log10(intensity)).max(0.0)
}

/// Wilson–Hilferty cube-root approximation for the `p_fraction` quantile of
/// Gamma(m, m) at large shape `m`.
fn wilson_hilferty_intensity(m: f64, p_fraction: f64) -> f64 {
    let z = normal_quantile(p_fraction);
    let term = 1.0 - 1.0 / (9.0 * m) + z / (3.0 * libm::sqrt(m));
    let cubed = term * term * term;
    cubed.max(1.0e-12)
}

/// Standard normal quantile Φ⁻¹(p) for p in (0, 1) via Acklam's rational
/// approximation refined with two Halley steps on `erfc`.
fn normal_quantile(p: f64) -> f64 {
    let q = if p <= 0.5 { p } else { 1.0 - p };
    let t = libm::sqrt(-2.0 * libm::log(q));
    let num = (0.010_328 * t + 0.802_853) * t + 2.515_517;
    let den = ((0.001_308 * t + 0.189_269) * t + 1.432_788) * t + 1.0;
    let mut x = -(t - num / den);
    if p > 0.5 {
        x = -x;
    }
    // Two Newton steps against exact `0.5 * erfc(-x / sqrt(2))`.
    let inv_sqrt_2pi = 0.398_942_280_401_432_7;
    for _ in 0..2 {
        let cdf = 0.5 * libm::erfc(-x / SQRT_2);
        let pdf = inv_sqrt_2pi * libm::exp(-0.5 * x * x);
        if pdf > 0.0 {
            x -= (cdf - p) / pdf;
        }
    }
    x
}

/// Inverts P(m, m * I) = target for I > 0 by bracketed Newton-bisection in u = ln(I).
fn invert_gamma_cdf(m: f64, target: f64) -> f64 {
    let guess = wilson_hilferty_intensity(m, target);
    let mut lo = libm::log(guess) - 2.0;
    let mut hi = libm::log(guess) + 2.0;
    while regularized_gamma_p(m, m * libm::exp(lo)) > target && lo > -60.0 {
        lo -= 4.0;
    }
    while regularized_gamma_p(m, m * libm::exp(hi)) < target && hi < 20.0 {
        hi += 4.0;
    }
    let ln_gamma_m = libm::lgamma(m);
    let mut u = 0.5 * (lo + hi);
    for _ in 0..64 {
        let intensity = libm::exp(u);
        let x = m * intensity;
        let cdf = regularized_gamma_p(m, x);
        let err = cdf - target;
        if libm::fabs(err) <= 1.0e-12 * target.max(1.0e-12) {
            return intensity;
        }
        if err < 0.0 {
            lo = u;
        } else {
            hi = u;
        }
        // dP/du = x^m * exp(-x) / Gamma(m)
        let d_cdf_du = libm::exp(m * libm::log(x) - x - ln_gamma_m);
        let newton = if d_cdf_du > 1.0e-300 {
            u - err / d_cdf_du
        } else {
            0.5 * (lo + hi)
        };
        u = if newton > lo && newton < hi {
            newton
        } else {
            0.5 * (lo + hi)
        };
        if hi - lo < 1.0e-13 {
            break;
        }
    }
    libm::exp(u)
}

/// Regularized lower incomplete gamma P(a, x) for a > 0, x >= 0.
pub(crate) fn regularized_gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        gamma_series(a, x)
    } else {
        1.0 - gamma_continued_fraction(a, x)
    }
}

/// Power series for P(a, x) when x < a + 1.
fn gamma_series(a: f64, x: f64) -> f64 {
    let ln_gamma_a = libm::lgamma(a);
    let mut ap = a;
    let mut term = 1.0 / a;
    let mut sum = term;
    for _ in 0..MAX_GAMMA_ITERS {
        ap += 1.0;
        term *= x / ap;
        sum += term;
        if libm::fabs(term) <= libm::fabs(sum) * 1.0e-15 {
            break;
        }
    }
    let scale = libm::exp(-x + a * libm::log(x) - ln_gamma_a);
    (sum * scale).clamp(0.0, 1.0)
}

/// Modified Lentz continued fraction for Q(a, x) = 1 - P(a, x) when x >= a + 1.
fn gamma_continued_fraction(a: f64, x: f64) -> f64 {
    let tiny = 1.0e-300;
    let ln_gamma_a = libm::lgamma(a);
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = if libm::fabs(b) < tiny {
        1.0 / tiny
    } else {
        1.0 / b
    };
    let mut h = d;
    let mut i = 1_usize;
    while i <= MAX_GAMMA_ITERS {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if libm::fabs(d) < tiny {
            d = tiny;
        }
        c = b + an / c;
        if libm::fabs(c) < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if libm::fabs(delta - 1.0) <= 1.0e-15 {
            break;
        }
        i = i.wrapping_add(1);
    }
    let scale = libm::exp(-x + a * libm::log(x) - ln_gamma_a);
    (scale * h).clamp(0.0, 1.0)
}
