//! Diffraction attenuation beyond the radio horizon: a weighted blend of
//! double knife-edge and Vogler smooth-Earth diffraction plus a clutter
//! factor (NTIA `DiffractionLoss`, `KnifeEdgeDiffraction`,
//! `SmoothEarthDiffraction`).

use crate::itm::complex::Complex;
use crate::itm::terrain::PathParameters;

/// Earth radius of the 4/3-Earth reference in Vogler (1964), m.
const A_0_M: f64 = 6370.0e3;
const THIRD: f64 = 1.0 / 3.0;

/// The terms every diffraction-region evaluation shares.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DiffractionContext {
    pub path: PathParameters,
    pub h_m: [f64; 2],
    pub z_g: Complex,
    pub a_e_m: f64,
    pub theta_los: f64,
    pub d_sml_m: f64,
    pub f_mhz: f64,
}

/// Δh(d), the terrain irregularity seen over distance `d_m`
/// ([ERL 79-ITS 67 eq. 3], in metres).
pub(crate) fn terrain_roughness(d_m: f64, delta_h_m: f64) -> f64 {
    delta_h_m * (1.0 - 0.8 * libm::exp(-d_m / 50.0e3))
}

/// σ_h, the RMS terrain deviation in the first Fresnel zone
/// ([ERL 79-ITS 67 eq. 3.6a]).
pub(crate) fn sigma_h(delta_h_m: f64) -> f64 {
    0.78 * delta_h_m * libm::exp(-0.5 * libm::pow(delta_h_m, 0.25))
}

/// A_d at distance `d_m` in the diffraction region
/// ([ERL 79-ITS 67 eqs 3.23, 3.38c]), point-to-point form (C ≈ 10).
pub(crate) fn diffraction_loss(ctx: &DiffractionContext, d_m: f64) -> f64 {
    let p = &ctx.path;
    let a_k = knife_edge(ctx, d_m);
    let a_se = smooth_earth(ctx, d_m);
    let sigma_h_d = sigma_h(terrain_roughness(ctx.d_sml_m, p.delta_h_m));
    let a_fo = 15.0_f64
        .min(5.0 * libm::log10(1.0 + 1.0e-5 * ctx.h_m[0] * ctx.h_m[1] * ctx.f_mhz * sigma_h_d));
    let delta_h_d = terrain_roughness(d_m, p.delta_h_m);
    let q = ctx.h_m[0] * ctx.h_m[1];
    let qk = p.h_e_m[0] * p.h_e_m[1] - q;
    let q = q + 10.0;
    let term1 = libm::sqrt(1.0 + qk / q);
    let d_ml = p.d_hzn_m[0] + p.d_hzn_m[1];
    let q = (term1 + (-ctx.theta_los * ctx.a_e_m + d_ml) / d_m)
        * (delta_h_d * ctx.f_mhz / 47.7).min(6283.2);
    let w = 25.1 / (25.1 + libm::sqrt(q));
    w * a_se + (1.0 - w) * a_k + a_fo
}

/// Double knife-edge attenuation at the two horizons ([TN101 eqs I.1,
/// I.7]).
fn knife_edge(ctx: &DiffractionContext, d_m: f64) -> f64 {
    let d_hzn = ctx.path.d_hzn_m;
    let d_ml = d_hzn[0] + d_hzn[1];
    let theta_nlos = d_m / ctx.a_e_m - ctx.theta_los;
    let d_nlos = d_m - d_ml;
    let v = |d_hzn_i: f64| {
        0.0795775 * (ctx.f_mhz / 47.7) * theta_nlos * theta_nlos * d_hzn_i * d_nlos
            / (d_nlos + d_hzn_i)
    };
    fresnel_integral(v(d_hzn[0])) + fresnel_integral(v(d_hzn[1]))
}

/// The knife-edge loss approximation of ITM, from v² ([TN101v2 eqs
/// III.24b, III.24c]).
fn fresnel_integral(v2: f64) -> f64 {
    if v2 < 5.76 {
        6.02 + 9.11 * libm::sqrt(v2) - 1.27 * v2
    } else {
        12.953 + 10.0 * libm::log10(v2)
    }
}

/// Vogler's three-radius smooth-Earth diffraction ([Vogler 1964];
/// [TN101 eq. 8.4]).
fn smooth_earth(ctx: &DiffractionContext, d_m: f64) -> f64 {
    let p = &ctx.path;
    let theta_nlos = d_m / ctx.a_e_m - ctx.theta_los;
    let d_ml = p.d_hzn_m[0] + p.d_hzn_m[1];
    let radii = [
        (d_m - d_ml) / (d_m / ctx.a_e_m - ctx.theta_los),
        0.5 * p.d_hzn_m[0] * p.d_hzn_m[0] / p.h_e_m[0],
        0.5 * p.d_hzn_m[1] * p.d_hzn_m[1] / p.h_e_m[1],
    ];
    let d_km = [
        radii[0] * theta_nlos / 1000.0,
        p.d_hzn_m[0] / 1000.0,
        p.d_hzn_m[1] / 1000.0,
    ];
    let f_third = libm::pow(ctx.f_mhz, THIRD);
    let z_g_abs = ctx.z_g.abs();
    // Per radius: K and x (km) of [Vogler 1964 eqs 2, 6a, 7a, Fig. 4].
    let term = |a_m: f64, d_km: f64| {
        let c_0 = libm::pow((4.0 / 3.0) * A_0_M / a_m, THIRD);
        let k = 0.017778 * c_0 * libm::pow(ctx.f_mhz, -THIRD) / z_g_abs;
        let b_0 = 1.607 - k;
        (k, b_0 * c_0 * c_0 * f_third * d_km)
    };
    let (_, x_path) = term(radii[0], d_km[0]);
    let (k_tx, x_tx) = term(radii[1], d_km[1]);
    let (k_rx, x_rx) = term(radii[2], d_km[2]);
    let x_0 = x_path + x_tx + x_rx;
    let g_x = 0.05751 * x_0 - 10.0 * libm::log10(x_0);
    g_x - height_function(x_tx, k_tx) - height_function(x_rx, k_rx) - 20.0
}

/// Vogler's height-gain function F(x, K).
fn height_function(x_km: f64, k: f64) -> f64 {
    if x_km < 200.0 {
        let w = -libm::log(k);
        if k < 1.0e-5 || x_km * w * w * w > 5495.0 {
            if x_km > 1.0 {
                17.372 * libm::log(x_km) - 117.0
            } else {
                -117.0
            }
        } else {
            2.5e-5 * x_km * x_km / k - 8.686 * w - 15.0
        }
    } else {
        let far = 0.05751 * x_km - 4.343 * libm::log(x_km);
        if x_km < 2000.0 {
            let w = 0.0134 * x_km * libm::exp(-0.005 * x_km);
            (1.0 - w) * far + w * (17.372 * libm::log(x_km) - 117.0)
        } else {
            far
        }
    }
}
