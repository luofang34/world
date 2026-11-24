//! Slant-path rain attenuation (ITU-R P.618-14 §2.2.1.1, ITU-R P.838-3,
//! ITU-R P.839-4) and hydrometeor cross-polar discrimination (§4.1).

/// Effective Earth radius accounting for refraction in rain path length, km.
const EFFECTIVE_EARTH_RADIUS_KM: f64 = 8_500.0;

/// ITU-R P.839-4 melting-layer offset added to 0°C isotherm height, km.
const RAIN_HEIGHT_OFFSET_KM: f64 = 0.36;

/// ITU-R P.838-3 Table 1: regression terms `(a_j, b_j, c_j)` for `k_H`.
const KH_TERMS: [(f64, f64, f64); 4] = [
    (-5.339_80, -0.100_08, 1.130_98),
    (-0.353_51, 1.269_70, 0.454_00),
    (-0.237_89, 0.860_36, 0.153_54),
    (-0.941_58, 0.645_52, 0.168_17),
];
const KH_LINEAR: (f64, f64) = (-0.189_61, 0.711_47);

/// ITU-R P.838-3 Table 2: regression terms `(a_j, b_j, c_j)` for `k_V`.
const KV_TERMS: [(f64, f64, f64); 4] = [
    (-3.805_95, 0.569_34, 0.810_61),
    (-3.449_65, -0.229_11, 0.510_59),
    (-0.399_02, 0.730_42, 0.118_99),
    (0.501_67, 1.073_19, 0.271_95),
];
const KV_LINEAR: (f64, f64) = (-0.163_98, 0.632_97);

/// ITU-R P.838-3 Table 3: regression terms `(a_j, b_j, c_j)` for `α_H`.
const AH_TERMS: [(f64, f64, f64); 5] = [
    (-0.143_18, 1.824_42, -0.551_87),
    (0.295_91, 0.775_64, 0.198_22),
    (0.321_77, 0.637_73, 0.131_64),
    (-5.376_10, -0.962_30, 1.478_28),
    (16.172_1, -3.299_80, 3.439_90),
];
const AH_LINEAR: (f64, f64) = (0.678_49, -1.955_37);

/// ITU-R P.838-3 Table 4: regression terms `(a_j, b_j, c_j)` for `α_V`.
const AV_TERMS: [(f64, f64, f64); 5] = [
    (-0.077_71, 2.338_40, -0.762_84),
    (0.567_27, 0.955_45, 0.540_39),
    (-0.202_38, 1.145_20, 0.268_09),
    (-48.299_1, 0.791_669, 0.116_226),
    (48.583_3, 0.791_459, 0.116_479),
];
const AV_LINEAR: (f64, f64) = (-0.053_739, 0.834_33);

/// Inputs for the ITU-R P.618-14 §2.2.1.1 slant-path rain attenuation model.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RainInputs {
    pub f_ghz: f64,
    pub el_deg: f64,
    pub tau_deg: f64,
    pub lat_deg: f64,
    pub station_alt_km: f64,
    pub h_0_km: f64,
    pub r_001_mm_h: f64,
    pub p_percent: f64,
}

/// Output of the P.618-14 rain attenuation calculation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RainOutcome {
    pub rain_001_db: f64,
    pub rain_db: f64,
    pub effective_path_length_km: f64,
    pub specific_attenuation_db_km: f64,
}

/// ITU-R P.838-3 `(k, α)` pair at frequency `f_ghz`, path elevation `el_deg`,
/// and polarisation tilt `tau_deg` (`0°` = H, `90°` = V, `45°` = circular).
pub(crate) fn specific_rain_coefficients(f_ghz: f64, el_deg: f64, tau_deg: f64) -> (f64, f64) {
    let lf = libm::log10(f_ghz);
    let k_h = eval_k(lf, &KH_TERMS, KH_LINEAR);
    let k_v = eval_k(lf, &KV_TERMS, KV_LINEAR);
    let a_h = eval_alpha(lf, &AH_TERMS, AH_LINEAR);
    let a_v = eval_alpha(lf, &AV_TERMS, AV_LINEAR);

    let cos_el = libm::cos(el_deg.to_radians());
    let factor = cos_el * cos_el * libm::cos(2.0 * tau_deg.to_radians());
    let k = 0.5 * (k_h + k_v + (k_h - k_v) * factor);
    let alpha = (k_h * a_h + k_v * a_v + (k_h * a_h - k_v * a_v) * factor) / (2.0 * k);
    (k, alpha)
}

fn eval_k(log10_f: f64, terms: &[(f64, f64, f64); 4], (mk, ck): (f64, f64)) -> f64 {
    let mut sum = mk * log10_f + ck;
    for &(a, b, c) in terms {
        let ratio = (log10_f - b) / c;
        sum += a * libm::exp(-(ratio * ratio));
    }
    libm::pow(10.0, sum)
}

fn eval_alpha(log10_f: f64, terms: &[(f64, f64, f64); 5], (ma, ca): (f64, f64)) -> f64 {
    let mut sum = ma * log10_f + ca;
    for &(a, b, c) in terms {
        let ratio = (log10_f - b) / c;
        sum += a * libm::exp(-(ratio * ratio));
    }
    sum
}

/// ITU-R P.618-14 §2.2.1.1 slant-path rain attenuation at probability `p_percent`.
pub(crate) fn rain_attenuation(inputs: RainInputs) -> RainOutcome {
    let RainInputs {
        f_ghz,
        el_deg,
        tau_deg,
        lat_deg,
        station_alt_km,
        h_0_km,
        r_001_mm_h,
        p_percent,
    } = inputs;
    let h_r = h_0_km + RAIN_HEIGHT_OFFSET_KM;
    let delta_h = h_r - station_alt_km;
    if delta_h <= 0.0 || r_001_mm_h <= 0.0 {
        return RainOutcome {
            rain_001_db: 0.0,
            rain_db: 0.0,
            effective_path_length_km: 0.0,
            specific_attenuation_db_km: 0.0,
        };
    }

    let sin_el = libm::sin(el_deg.to_radians());
    let cos_el = libm::cos(el_deg.to_radians());
    let l_s = if el_deg >= 5.0 {
        delta_h / sin_el
    } else {
        2.0 * delta_h
            / (sin_el + libm::sqrt(sin_el * sin_el + 2.0 * delta_h / EFFECTIVE_EARTH_RADIUS_KM))
    };
    let l_g = l_s * cos_el;

    let (k, alpha) = specific_rain_coefficients(f_ghz, el_deg, tau_deg);
    let gamma_r = k * libm::pow(r_001_mm_h, alpha);

    let r_001 = 1.0
        / (1.0 + 0.78 * libm::sqrt((l_g * gamma_r) / f_ghz) - 0.38 * (1.0 - libm::exp(-2.0 * l_g)));

    let l_r = if l_g * r_001 <= 0.0 || el_deg >= 90.0 {
        delta_h / sin_el
    } else {
        let zeta_deg = libm::atan(delta_h / (l_g * r_001)).to_degrees();
        if zeta_deg > el_deg {
            (l_g * r_001) / cos_el
        } else {
            delta_h / sin_el
        }
    };

    let abs_lat = libm::fabs(lat_deg);
    let chi = if abs_lat < 36.0 { 36.0 - abs_lat } else { 0.0 };
    let v_001 = 1.0
        / (1.0
            + libm::sqrt(sin_el)
                * (31.0 * (1.0 - libm::exp(-(el_deg / (1.0 + chi)))) * libm::sqrt(l_r * gamma_r)
                    / (f_ghz * f_ghz)
                    - 0.45));

    let l_e = l_r * v_001;
    let a_001 = (gamma_r * l_e).max(0.0);
    let rain_db = scale_rain_percentage(a_001, p_percent, abs_lat, el_deg, sin_el);

    RainOutcome {
        rain_001_db: a_001,
        rain_db,
        effective_path_length_km: l_e,
        specific_attenuation_db_km: gamma_r,
    }
}

/// Scales `A_0.01` to exceedance probability `p_percent` (P.618-14 §2.2.1.1 / §2.5).
fn scale_rain_percentage(
    a_001: f64,
    p_percent: f64,
    abs_lat: f64,
    el_deg: f64,
    sin_el: f64,
) -> f64 {
    if p_percent > 5.0 || a_001 <= 0.0 {
        return 0.0;
    }
    let beta = if p_percent >= 1.0 || abs_lat >= 36.0 {
        0.0
    } else if el_deg >= 25.0 {
        -0.005 * (abs_lat - 36.0)
    } else {
        -0.005 * (abs_lat - 36.0) + 1.8 - 4.25 * sin_el
    };
    let exp = -(0.655 + 0.033 * libm::log(p_percent)
        - 0.045 * libm::log(a_001)
        - beta * (1.0 - p_percent) * sin_el);
    (a_001 * libm::pow(p_percent / 0.01, exp)).max(0.0)
}

/// ITU-R P.618-14 §4.1 hydrometeor cross-polar discrimination (XPD), dB.
///
/// Defined for `6 <= f <= 55 GHz`, `elevation <= 60 deg`, `p <= 1 %`, and `A_p > 0`.
pub(crate) fn cross_polar_discrimination_db(
    f_ghz: f64,
    el_deg: f64,
    tau_deg: f64,
    p_percent: f64,
    a_p_db: f64,
) -> Option<f64> {
    if !(6.0..=55.0).contains(&f_ghz) || el_deg > 60.0 || p_percent > 1.0 || a_p_db <= 0.0 {
        return None;
    }
    let lf = libm::log10(f_ghz);
    let c_f = if f_ghz < 9.0 {
        60.0 * lf - 28.3
    } else if f_ghz < 36.0 {
        26.0 * lf + 4.1
    } else {
        35.9 * lf - 11.3
    };
    let v_f = if f_ghz < 9.0 {
        30.8 * libm::pow(f_ghz, -0.21)
    } else if f_ghz < 20.0 {
        12.8 * libm::pow(f_ghz, 0.19)
    } else if f_ghz < 40.0 {
        22.6
    } else {
        13.0 * libm::pow(f_ghz, 0.15)
    };
    let c_a = v_f * libm::log10(a_p_db);
    let c_tau = -10.0 * libm::log10(1.0 - 0.484 * (1.0 + libm::cos(4.0 * tau_deg.to_radians())));
    let c_theta = -40.0 * libm::log10(libm::cos(el_deg.to_radians()));
    let sigma = (-5.0 * libm::log10(p_percent)).max(0.0);
    let c_sigma = 0.0053 * sigma * sigma;

    let xpd_rain = c_f - c_a + c_tau + c_theta + c_sigma;
    let c_ice = xpd_rain.max(0.0) * (0.3 + 0.1 * libm::log10(p_percent)) * 0.5;
    Some(xpd_rain - c_ice)
}
