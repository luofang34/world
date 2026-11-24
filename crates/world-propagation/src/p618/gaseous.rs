//! Slant-path atmospheric gaseous attenuation (ITU-R P.676 Annex 2).

/// Dry-air and water-vapour slant gaseous attenuation breakdown, dB.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GaseousOutcome {
    pub oxygen_db: f64,
    pub water_vapour_db: f64,
    pub total_db: f64,
}

/// Evaluates slant-path gaseous attenuation from 1 to 55 GHz at elevation `el_deg`.
pub(crate) fn gaseous_attenuation(
    f_ghz: f64,
    el_deg: f64,
    pressure_hpa: f64,
    temperature_k: f64,
    water_vapour_density_g_m3: f64,
) -> GaseousOutcome {
    let rp = pressure_hpa / 1013.25;
    let rt = 288.15 / temperature_k;
    let gamma_o = specific_oxygen_db_km(f_ghz, rp, rt);
    let gamma_w = specific_water_vapour_db_km(f_ghz, water_vapour_density_g_m3);
    let h_o = equivalent_height_oxygen_km(f_ghz, rp);
    let h_w = equivalent_height_water_vapour_km(f_ghz, rp);

    let sin_el = libm::sin(el_deg.to_radians());
    let oxygen_db = (gamma_o * h_o) / sin_el;
    let water_vapour_db = (gamma_w * h_w) / sin_el;
    GaseousOutcome {
        oxygen_db,
        water_vapour_db,
        total_db: oxygen_db + water_vapour_db,
    }
}

/// Dry-air specific attenuation `γ_o`, dB/km (P.676-10 Annex 2 for 1 to 60 GHz).
fn specific_oxygen_db_km(f_ghz: f64, rp: f64, rt: f64) -> f64 {
    if f_ghz <= 54.0 {
        let xi1 = phi(rp, rt, 0.0717, -1.8132, 0.0156, -1.6515);
        let xi2 = phi(rp, rt, 0.5146, -4.6368, -0.1921, -5.7416);
        let xi3 = phi(rp, rt, 0.3414, -6.5851, 0.2130, -8.5854);
        let t1 = (7.2 * libm::pow(rt, 2.8)) / (f_ghz * f_ghz + 0.34 * rp * rp * libm::pow(rt, 1.6));
        let t2 = (0.62 * xi3) / (libm::pow(54.0 - f_ghz, 1.16 * xi1) + 0.83 * xi2);
        return (t1 + t2) * f_ghz * f_ghz * rp * rp * 1.0e-3;
    }

    // 54 < f <= 60 GHz Lagrange log-interpolation across 54, 58, 60 GHz anchors.
    let g54 = 2.192 * phi(rp, rt, 1.8286, -1.9487, 0.4051, -2.8509);
    let g58 = 12.59 * phi(rp, rt, 1.0045, 3.5610, 0.1588, 1.2834);
    let g60 = 15.0 * phi(rp, rt, 0.9003, 4.1335, 0.0427, 1.6088);
    let exponent = (libm::log(g54) / 24.0) * (f_ghz - 58.0) * (f_ghz - 60.0)
        - (libm::log(g58) / 8.0) * (f_ghz - 54.0) * (f_ghz - 60.0)
        + (libm::log(g60) / 12.0) * (f_ghz - 54.0) * (f_ghz - 58.0);
    libm::exp(exponent)
}

fn phi(rp: f64, rt: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    libm::pow(rp, a) * libm::pow(rt, b) * libm::exp(c * (1.0 - rp) + d * (1.0 - rt))
}

/// Water-vapour specific attenuation `γ_w`, dB/km (P.676-5 Annex 2 curve fit).
fn specific_water_vapour_db_km(f_ghz: f64, rho: f64) -> f64 {
    if rho <= 0.0 {
        return 0.0;
    }
    let d22 = f_ghz - 22.2;
    let d183 = f_ghz - 183.3;
    let d325 = f_ghz - 325.4;
    let bracket = 0.050
        + 0.0021 * rho
        + 3.6 / (d22 * d22 + 8.5)
        + 10.6 / (d183 * d183 + 9.0)
        + 8.9 / (d325 * d325 + 26.3);
    bracket * f_ghz * f_ghz * rho * 1.0e-4
}

/// Dry-air equivalent scale height `h_o`, km (P.676-10 Annex 2).
fn equivalent_height_oxygen_km(f_ghz: f64, rp: f64) -> f64 {
    let t1_exp = (f_ghz - 59.7) / (2.87 + 12.4 * libm::exp(-7.9 * rp));
    let t1 = (4.64 / (1.0 + 0.066 * libm::pow(rp, -2.3))) * libm::exp(-(t1_exp * t1_exp));
    let d118 = f_ghz - 118.75;
    let t2 = (0.14 * libm::exp(2.12 * rp)) / (d118 * d118 + 0.031 * libm::exp(2.2 * rp));
    let num = f_ghz * (-0.0247 + 0.0001 * f_ghz + 1.61e-6 * f_ghz * f_ghz);
    let den = 1.0 - 0.0169 * f_ghz + 4.1e-5 * f_ghz * f_ghz + 3.2e-7 * f_ghz * f_ghz * f_ghz;
    let t3 = (0.0114 / (1.0 + 0.14 * libm::pow(rp, -2.6))) * (num / den);
    (6.1 / (1.0 + 0.17 * libm::pow(rp, -1.1))) * (1.0 + t1 + t2 + t3)
}

/// Water-vapour equivalent scale height `h_w`, km (P.676-10 Annex 2).
fn equivalent_height_water_vapour_km(f_ghz: f64, rp: f64) -> f64 {
    let sigma_w = 1.013 / (1.0 + libm::exp(-8.6 * (rp - 0.57)));
    let d22 = f_ghz - 22.235;
    let d183 = f_ghz - 183.31;
    let d325 = f_ghz - 325.1;
    1.66 * (1.0
        + (1.39 * sigma_w) / (d22 * d22 + 2.56 * sigma_w)
        + (3.37 * sigma_w) / (d183 * d183 + 4.69 * sigma_w)
        + (1.58 * sigma_w) / (d325 * d325 + 2.89 * sigma_w))
}
