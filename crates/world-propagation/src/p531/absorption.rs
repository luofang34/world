//! Ionospheric D/E-region and auroral absorption (ITU-R P.531-15 §3).

use crate::units::Percentage;

use super::{EARTH_RADIUS_M, IonosphericZone};

/// Representative D/E-region absorbing shell height, m.
const D_REGION_HEIGHT_M: f64 = 100_000.0;

/// Normal mid-latitude vertical absorption at 30 MHz, dB (P.531 §3.1: 0.2 to 0.5 dB).
const MID_LAT_30MHZ_DB: f64 = 0.35;

/// Polar-cap enhancement factor relative to auroral absorption when not in a
/// separate PCA event model.
const POLAR_CAP_FACTOR: f64 = 1.25;

/// Reference frequency for auroral absorption Table 2, Hz (127 MHz).
const AURORAL_REF_HZ: f64 = 127.0e6;

/// ITU-R P.531 Table 2 auroral absorption at 127 MHz: `(percentage, db_20deg, db_5deg)`.
const AURORAL_TABLE_127MHZ: [(f64, f64, f64); 5] = [
    (0.1, 1.5, 2.9),
    (1.0, 0.9, 1.7),
    (2.0, 0.7, 1.4),
    (5.0, 0.6, 1.1),
    (50.0, 0.2, 0.4),
];

/// Thin-shell obliquity factor at D/E-region height `D_REGION_HEIGHT_M`.
pub(crate) fn d_region_obliquity(elevation_deg: f64) -> f64 {
    if elevation_deg >= 90.0 {
        return 1.0;
    }
    let cos_el = libm::cos(elevation_deg.to_radians());
    let ratio = (EARTH_RADIUS_M / (EARTH_RADIUS_M + D_REGION_HEIGHT_M)) * cos_el;
    1.0 / libm::sqrt((1.0 - ratio * ratio).max(1.0e-12))
}

/// Computes ionospheric absorption attenuation, dB (`1/f²` and `sec i` scaling).
pub(crate) fn ionospheric_absorption_db(
    zone: IonosphericZone,
    elevation_deg: f64,
    frequency_hz: f64,
    vtec_tecu: f64,
    percentage: Percentage,
) -> f64 {
    if vtec_tecu <= 0.0
        && matches!(
            zone,
            IonosphericZone::LowLatitude | IonosphericZone::MidLatitude
        )
    {
        return 0.0;
    }
    let obliq = d_region_obliquity(elevation_deg);
    match zone {
        IonosphericZone::LowLatitude | IonosphericZone::MidLatitude => {
            let scale = 30.0e6 / frequency_hz;
            MID_LAT_30MHZ_DB * scale * scale * obliq
        }
        IonosphericZone::Auroral | IonosphericZone::PolarCap => {
            let base_127 = auroral_table_127mhz_db(percentage.percent(), obliq);
            let scale = AURORAL_REF_HZ / frequency_hz;
            let mult = if matches!(zone, IonosphericZone::PolarCap) {
                POLAR_CAP_FACTOR
            } else {
                1.0
            };
            base_127 * scale * scale * mult
        }
    }
}

/// Interpolates ITU-R P.531 Table 2 in log(p) and scales across elevation via
/// D-region obliquity relative to 20 deg.
fn auroral_table_127mhz_db(percent: f64, obliq: f64) -> f64 {
    let obliq_20 = d_region_obliquity(20.0);
    let obliq_5 = d_region_obliquity(5.0);

    let interp_row = |db_20: f64, db_5: f64| {
        if obliq < obliq_20 {
            db_20 * (obliq / obliq_20)
        } else if obliq <= obliq_5 {
            let t = (obliq - obliq_20) / (obliq_5 - obliq_20);
            db_20 + t * (db_5 - db_20)
        } else {
            db_5 * (obliq / obliq_5)
        }
    };

    let lp = libm::log10(percent);
    for pair in AURORAL_TABLE_127MHZ.windows(2) {
        let Some(&(p0, a0, b0)) = pair.first() else {
            continue;
        };
        let Some(&(p1, a1, b1)) = pair.get(1) else {
            continue;
        };
        if percent <= p1 {
            let lp0 = libm::log10(p0);
            let lp1 = libm::log10(p1);
            let frac = (lp - lp0) / (lp1 - lp0);
            let v0 = interp_row(a0, b0);
            let v1 = interp_row(a1, b1);
            return (v0 + frac * (v1 - v0)).max(0.0);
        }
    }
    // Tail beyond 50 % decays toward 0 at 100 %.
    let [.., (_, last_20, last_5)] = AURORAL_TABLE_127MHZ;
    let v50 = interp_row(last_20, last_5);
    let tail = ((100.0 - percent) / 50.0).clamp(0.0, 1.0);
    v50 * tail
}
