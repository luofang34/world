//! One evaluation's inputs, in the units and forms the formulation uses.

use crate::input::{GeomagneticActivity, MsisInput};
use crate::math::{cos, sin};
use crate::switches::{Factors, Switches};

/// Degrees to radians, as the reference rounds it.
pub(crate) const DGTR: f64 = 1.745_33E-2;
/// Radians per day of the year, as the reference rounds it.
pub(crate) const DR: f64 = 1.721_42E-2;
/// Radians per hour of local time, as the reference rounds it.
pub(crate) const HR: f64 = 0.2618;
/// Radians per second of universal time, as the reference rounds it.
pub(crate) const SR: f64 = 7.2722E-5;

/// Surface gravity and the effective Earth radius at a latitude.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Gravity {
    /// Surface gravity, cm/s².
    pub(crate) gsurf: f64,
    /// Effective radius, km.
    pub(crate) re: f64,
}

/// Associated Legendre functions of the latitude and the harmonics of the
/// local time.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Harmonics {
    /// `plg[m][n]`: order `m`, degree `n`, unnormalized.
    pub(crate) plg: [[f64; 9]; 4],
    pub(crate) stloc: f64,
    pub(crate) ctloc: f64,
    pub(crate) s2tloc: f64,
    pub(crate) c2tloc: f64,
    pub(crate) s3tloc: f64,
    pub(crate) c3tloc: f64,
}

/// Everything one evaluation reads and never changes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Call {
    /// Day of the year.
    pub(crate) doy: f64,
    /// Seconds of the UTC day.
    pub(crate) sec: f64,
    /// Geodetic altitude, km.
    pub(crate) alt: f64,
    /// Geodetic latitude, degrees.
    pub(crate) g_lat: f64,
    /// Longitude, degrees east.
    pub(crate) g_long: f64,
    /// Local apparent solar time, hours.
    pub(crate) lst: f64,
    /// 81-day mean F10.7.
    pub(crate) f107a: f64,
    /// Previous-day F10.7.
    pub(crate) f107: f64,
    /// Daily Ap.
    pub(crate) ap: f64,
    /// The Ap history, in the reference's array order.
    pub(crate) ap_history: [f64; 7],
    /// The 81-day mean F10.7 less 150.
    pub(crate) dfa: f64,
    pub(crate) factors: Factors,
    pub(crate) harmonics: Harmonics,
    pub(crate) gravity: Gravity,
}

/// The geomagnetic activity functions, which each upper-thermosphere
/// parameter set evaluates with its own coefficients and the lower
/// atmosphere then reads as the last set left them.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Carry {
    /// The daily-Ap activity function.
    pub(crate) apdf: f64,
    /// The Ap-history activity function.
    pub(crate) apt0: f64,
}

impl Gravity {
    /// The reference's `GLATF`: gravity and effective radius at `lat`
    /// degrees.
    pub(crate) fn at_latitude(lat: f64) -> Self {
        let c2 = cos(2.0 * DGTR * lat);
        let gsurf = 980.616 * (1.0 - 0.002_637_3 * c2);
        let re = 2.0 * gsurf / (3.085_462E-6 + 2.27E-9 * c2) * 1.0E-5;
        Self { gsurf, re }
    }

    /// Geopotential height difference of `zz` from `zl`, km.
    pub(crate) fn zeta(&self, zz: f64, zl: f64) -> f64 {
        (zz - zl) * (self.re + zl) / (self.re + zz)
    }
}

impl Harmonics {
    pub(crate) fn new(g_lat: f64, tloc: f64) -> Self {
        let c = sin(g_lat * DGTR);
        let s = cos(g_lat * DGTR);
        let c2 = c * c;
        let c4 = c2 * c2;
        let s2 = s * s;
        let mut plg = [[0.0; 9]; 4];
        plg[0][1] = c;
        plg[0][2] = 0.5 * (3.0 * c2 - 1.0);
        plg[0][3] = 0.5 * (5.0 * c * c2 - 3.0 * c);
        plg[0][4] = (35.0 * c4 - 30.0 * c2 + 3.0) / 8.0;
        plg[0][5] = (63.0 * c2 * c2 * c - 70.0 * c2 * c + 15.0 * c) / 8.0;
        plg[0][6] = (11.0 * c * plg[0][5] - 5.0 * plg[0][4]) / 6.0;
        plg[1][1] = s;
        plg[1][2] = 3.0 * c * s;
        plg[1][3] = 1.5 * (5.0 * c2 - 1.0) * s;
        plg[1][4] = 2.5 * (7.0 * c2 * c - 3.0 * c) * s;
        plg[1][5] = 1.875 * (21.0 * c4 - 14.0 * c2 + 1.0) * s;
        plg[1][6] = (11.0 * c * plg[1][5] - 6.0 * plg[1][4]) / 5.0;
        plg[2][2] = 3.0 * s2;
        plg[2][3] = 15.0 * s2 * c;
        plg[2][4] = 7.5 * (7.0 * c2 - 1.0) * s2;
        plg[2][5] = 3.0 * c * plg[2][4] - 2.0 * plg[2][3];
        plg[2][6] = (11.0 * c * plg[2][5] - 7.0 * plg[2][4]) / 4.0;
        plg[2][7] = (13.0 * c * plg[2][6] - 8.0 * plg[2][5]) / 5.0;
        plg[3][3] = 15.0 * s2 * s;
        plg[3][4] = 105.0 * s2 * s * c;
        plg[3][5] = (9.0 * c * plg[3][4] - 7.0 * plg[3][3]) / 2.0;
        plg[3][6] = (11.0 * c * plg[3][5] - 8.0 * plg[3][4]) / 3.0;
        Self {
            plg,
            stloc: sin(HR * tloc),
            ctloc: cos(HR * tloc),
            s2tloc: sin(2.0 * HR * tloc),
            c2tloc: cos(2.0 * HR * tloc),
            s3tloc: sin(3.0 * HR * tloc),
            c3tloc: cos(3.0 * HR * tloc),
        }
    }
}

impl Call {
    pub(crate) fn new(input: &MsisInput, switches: &Switches) -> Self {
        let history = match input.geomagnetic {
            GeomagneticActivity::DailyAp(_) => None,
            GeomagneticActivity::History(history) => Some(history.array()),
        };
        let factors = switches.factors(history.is_some());
        let g_lat = input.position.latitude_deg;
        // With the time-independent term off the reference takes gravity
        // at 45 degrees.
        let gravity_latitude = if factors.sw[2] == 0.0 { 45.0 } else { g_lat };
        Self {
            doy: f64::from(input.day_of_year),
            sec: input.ut_seconds,
            alt: input.position.ellipsoidal_height_m / 1000.0,
            g_lat,
            g_long: input.position.longitude_deg,
            lst: input.local_solar_time_h,
            f107a: input.f107_81_day_mean,
            f107: input.f107_previous_day,
            ap: input.daily_ap(),
            ap_history: history.unwrap_or([0.0; 7]),
            dfa: input.f107_81_day_mean - 150.0,
            factors,
            harmonics: Harmonics::new(g_lat, input.local_solar_time_h),
            gravity: Gravity::at_latitude(gravity_latitude),
        }
    }
}
