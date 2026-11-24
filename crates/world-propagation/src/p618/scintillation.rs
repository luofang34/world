//! Tropospheric amplitude scintillation fading (ITU-R P.618-14 §2.4.1).

/// Effective turbulent layer height `h_L`, m.
const TURBULENT_HEIGHT_M: f64 = 1_000.0;

/// Slant-path tropospheric scintillation fade depth `A_S(p)`, dB.
pub(crate) fn scintillation_attenuation_db(
    f_ghz: f64,
    el_deg: f64,
    antenna_diameter_m: f64,
    antenna_efficiency: f64,
    wet_refractivity_n: f64,
    p_percent: f64,
) -> f64 {
    let sigma_ref = 3.6e-3 + 1.0e-4 * wet_refractivity_n;
    let sin_el = libm::sin(el_deg.to_radians());
    let path_len = (2.0 * TURBULENT_HEIGHT_M) / (libm::sqrt(sin_el * sin_el + 2.35e-4) + sin_el);
    let d_eff = libm::sqrt(antenna_efficiency) * antenna_diameter_m;
    let x = 1.22 * d_eff * d_eff * (f_ghz / path_len);

    let g_sq = if x <= 0.0 {
        1.0
    } else {
        3.86 * libm::pow(x * x + 1.0, 11.0 / 12.0) * libm::sin((11.0 / 6.0) * libm::atan(1.0 / x))
            - 7.08 * libm::pow(x, 5.0 / 6.0)
    };
    if g_sq <= 0.0 {
        return 0.0;
    }
    let g_x = libm::sqrt(g_sq);
    let sigma = sigma_ref * libm::pow(f_ghz, 7.0 / 12.0) * g_x / libm::pow(sin_el, 1.2);
    let log_p = libm::log10(p_percent);
    let a_p = -0.061 * log_p * log_p * log_p + 0.072 * log_p * log_p - 1.71 * log_p + 3.0;
    (a_p.max(0.0) * sigma).max(0.0)
}
