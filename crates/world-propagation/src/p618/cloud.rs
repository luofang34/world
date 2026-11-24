//! Slant-path cloud attenuation via Rayleigh scattering and double-Debye
//! liquid-water permittivity (ITU-R P.840-9 §2–3).

/// Standard liquid-water reference temperature for reduced columnar content
/// `L_red`, K (`0 °C`).
const CLOUD_REF_TEMP_K: f64 = 273.15;

/// Cloud liquid-water specific attenuation coefficient `K_l(f, T)`,
/// `(dB/km) / (g/m³)` == `dB / (kg/m²)`.
pub(crate) fn specific_cloud_attenuation_kl(f_ghz: f64, temperature_k: f64) -> f64 {
    let theta = 300.0 / temperature_k;
    let theta_m1 = theta - 1.0;
    let eps0 = 77.66 + 103.3 * theta_m1;
    let eps1 = 0.0671 * eps0;
    let eps2 = 3.52;

    let fp = 20.20 - 146.0 * theta_m1 + 316.0 * theta_m1 * theta_m1;
    let fs = 39.8 * fp;

    let r_p = f_ghz / fp;
    let r_s = f_ghz / fs;
    let den_p = 1.0 + r_p * r_p;
    let den_s = 1.0 + r_s * r_s;

    let eps_real = (eps0 - eps1) / den_p + (eps1 - eps2) / den_s + eps2;
    let eps_imag = ((eps0 - eps1) * r_p) / den_p + ((eps1 - eps2) * r_s) / den_s;
    let eta = (2.0 + eps_real) / eps_imag;
    (0.819 * f_ghz) / (eps_imag * (1.0 + eta * eta))
}

/// Slant-path cloud attenuation `A_C`, dB, from reduced columnar liquid water
/// content `l_red_kg_m2` (`L_red` at `273.15 K`).
pub(crate) fn cloud_attenuation_db(f_ghz: f64, el_deg: f64, l_red_kg_m2: f64) -> f64 {
    if l_red_kg_m2 <= 0.0 {
        return 0.0;
    }
    let k_l = specific_cloud_attenuation_kl(f_ghz, CLOUD_REF_TEMP_K);
    let sin_el = libm::sin(el_deg.to_radians());
    (l_red_kg_m2 * k_l) / sin_el
}
