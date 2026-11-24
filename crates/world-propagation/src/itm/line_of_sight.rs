//! Line-of-sight attenuation: a two-ray model over rough ground blended
//! with the extended diffraction line (NTIA `LineOfSightLoss`).

use core::f64::consts::PI;

use crate::itm::complex::Complex;
use crate::itm::diffraction::{sigma_h, terrain_roughness};

/// The diffraction line A = M_d d + A_d0 and the path constants the
/// line-of-sight blend uses.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineOfSightContext {
    pub h_e_m: [f64; 2],
    pub z_g: Complex,
    pub delta_h_m: f64,
    pub m_d: f64,
    pub a_d0_db: f64,
    pub d_sml_m: f64,
    pub f_mhz: f64,
}

/// A_los at `d_m` ([Algorithm eqs 4.46 to 4.50]).
pub(crate) fn line_of_sight_loss(ctx: &LineOfSightContext, d_m: f64) -> f64 {
    let sigma_h_d = sigma_h(terrain_roughness(d_m, ctx.delta_h_m));
    let wn = ctx.f_mhz / 47.7;
    let h_sum = ctx.h_e_m[0] + ctx.h_e_m[1];
    let sin_psi = h_sum / libm::sqrt(d_m * d_m + h_sum * h_sum);
    let sin = Complex::new(sin_psi, 0.0);
    let roughness = libm::exp(-(wn * sigma_h_d * sin_psi).min(10.0));
    let mut r_e = ((sin - ctx.z_g) / (sin + ctx.z_g)).scale(roughness);
    let q = r_e.norm_sqr();
    if q < 0.25 || q < sin_psi {
        r_e = r_e.scale(libm::sqrt(sin_psi / q));
    }
    let mut delta_phi = wn * 2.0 * ctx.h_e_m[0] * ctx.h_e_m[1] / d_m;
    if delta_phi > PI / 2.0 {
        delta_phi = PI - (PI / 2.0) * (PI / 2.0) / delta_phi;
    }
    let rr = Complex::new(libm::cos(delta_phi), -libm::sin(delta_phi)) + r_e;
    let a_t = -10.0 * libm::log10(rr.norm_sqr());
    let a_d = ctx.m_d * d_m + ctx.a_d0_db;
    let w = 1.0 / (1.0 + ctx.f_mhz * ctx.delta_h_m / 10.0e3_f64.max(ctx.d_sml_m));
    w * a_t + (1.0 - w) * a_d
}
