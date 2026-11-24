//! What an MSIS-class model is asked.

use world_core::GeodeticPosition;

use crate::ThermosphereError;

/// The geomagnetic activity history the Ap-history formulation takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApHistory {
    /// The daily Ap. Read only when
    /// [`crate::Effect::GeomagneticActivity`] is not on.
    pub daily: f64,
    /// The 3-hour ap index of the current time.
    pub current: f64,
    /// The 3-hour ap index 3 hours before.
    pub hours_3_before: f64,
    /// The 3-hour ap index 6 hours before.
    pub hours_6_before: f64,
    /// The 3-hour ap index 9 hours before.
    pub hours_9_before: f64,
    /// The mean of the eight 3-hour ap indices from 12 to 33 hours before.
    pub mean_12_to_33_hours_before: f64,
    /// The mean of the eight 3-hour ap indices from 36 to 57 hours before.
    pub mean_36_to_57_hours_before: f64,
}

/// Geomagnetic activity, in one of the two forms the model defines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeomagneticActivity {
    /// The daily Ap index alone.
    DailyAp(f64),
    /// The 3-hourly ap history, which resolves a storm's onset and decay.
    History(ApHistory),
}

/// Where, when and under what solar and geomagnetic activity the upper
/// atmosphere is asked about.
///
/// The model does not derive one input from another: the local solar time
/// is its own field, so a caller studying one variation in isolation can
/// hold the others fixed. [`MsisInput::apparent_local_solar_time_h`] gives
/// the value consistent with a universal time and longitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MsisInput {
    /// The position. Its ellipsoidal height is the model's geodetic
    /// altitude.
    pub position: GeodeticPosition,
    /// The day of the year, 1 on January 1.
    pub day_of_year: u16,
    /// Seconds of the UTC day.
    pub ut_seconds: f64,
    /// Local apparent solar time, hours.
    pub local_solar_time_h: f64,
    /// The 10.7 cm solar radio flux of the previous day, in solar flux
    /// units, as observed at the Earth's actual distance from the Sun.
    pub f107_previous_day: f64,
    /// The 81-day mean of that flux, centred on the day.
    pub f107_81_day_mean: f64,
    /// Geomagnetic activity.
    pub geomagnetic: GeomagneticActivity,
}

impl ApHistory {
    /// The history in the reference's array order.
    pub(crate) const fn array(&self) -> [f64; 7] {
        [
            self.daily,
            self.current,
            self.hours_3_before,
            self.hours_6_before,
            self.hours_9_before,
            self.mean_12_to_33_hours_before,
            self.mean_36_to_57_hours_before,
        ]
    }
}

impl MsisInput {
    /// The local solar time the reference recommends for a physically
    /// consistent call: `ut_seconds / 3600 + longitude_deg / 15` hours,
    /// not wrapped into a day.
    pub fn apparent_local_solar_time_h(ut_seconds: f64, longitude_deg: f64) -> f64 {
        ut_seconds / 3600.0 + longitude_deg / 15.0
    }

    /// The daily Ap, whichever form the activity is given in.
    pub(crate) const fn daily_ap(&self) -> f64 {
        match self.geomagnetic {
            GeomagneticActivity::DailyAp(ap) => ap,
            GeomagneticActivity::History(history) => history.daily,
        }
    }

    /// Refuses an input no model is defined for.
    pub(crate) fn validate(&self) -> Result<(), ThermosphereError> {
        let invalid = |quantity, value| Err(ThermosphereError::InvalidInput { quantity, value });
        if !(-90.0..=90.0).contains(&self.position.latitude_deg) {
            return invalid("position.latitude_deg", self.position.latitude_deg);
        }
        if !self.position.longitude_deg.is_finite() {
            return invalid("position.longitude_deg", self.position.longitude_deg);
        }
        if !self.position.ellipsoidal_height_m.is_finite() {
            return invalid(
                "position.ellipsoidal_height_m",
                self.position.ellipsoidal_height_m,
            );
        }
        if !(1..=366).contains(&self.day_of_year) {
            return invalid("day_of_year", f64::from(self.day_of_year));
        }
        if !self.ut_seconds.is_finite() {
            return invalid("ut_seconds", self.ut_seconds);
        }
        if !self.local_solar_time_h.is_finite() {
            return invalid("local_solar_time_h", self.local_solar_time_h);
        }
        if !(self.f107_previous_day.is_finite() && self.f107_previous_day > 0.0) {
            return invalid("f107_previous_day", self.f107_previous_day);
        }
        if !(self.f107_81_day_mean.is_finite() && self.f107_81_day_mean > 0.0) {
            return invalid("f107_81_day_mean", self.f107_81_day_mean);
        }
        let ap = match self.geomagnetic {
            GeomagneticActivity::DailyAp(ap) => [ap; 7],
            GeomagneticActivity::History(history) => history.array(),
        };
        if let Some(&bad) = ap.iter().find(|ap| !(ap.is_finite() && **ap >= 0.0)) {
            return invalid("geomagnetic", bad);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
