//! Daily space-weather domain types, validation, and precedence merging.

use world_core::{GeodeticPosition, Provenance};

use crate::{GeomagneticActivity, MsisInput};

use super::calendar::CalendarDate;
use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};

/// Dataset family a daily space-weather record originated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpaceWeatherSource {
    /// GFZ Helmholtz Centre for Geosciences Kp/ap/Ap/F10.7 index file.
    Gfz,
    /// NOAA SWPC Daily Solar Data (`DSD.txt`).
    NoaaDailySolarData,
    /// NOAA SWPC 10.7 cm solar radio flux JSON (`f107_cm_flux.json`).
    NoaaF107Json,
    /// CelesTrak space-weather CSV (`SW-Last5Years.csv` / `SW-All.csv`).
    CelestrakCsv,
    /// CelesTrak legacy CSSI fixed-width text (`SW-All.txt`).
    CelestrakLegacy,
    /// Merged from more than one source record for the same date.
    Merged,
}

impl SpaceWeatherSource {
    /// Attribution and licensing summary for this data source.
    pub const fn attribution(self) -> &'static str {
        match self {
            Self::Gfz => super::gfz::GFZ_ATTRIBUTION,
            Self::NoaaDailySolarData => {
                "NOAA Space Weather Prediction Center (public domain; U.S. Government work)"
            }
            Self::NoaaF107Json => {
                "NOAA Space Weather Prediction Center / DRAO Penticton 2800 MHz flux (public domain; U.S. Government work)"
            }
            Self::CelestrakCsv | Self::CelestrakLegacy => {
                "CelesTrak Space Weather Data (derived from GFZ, DRAO/NRCan, NOAA SWPC, SILSO, and NASA; subject to CelesTrak usage policy)"
            }
            Self::Merged => {
                "Merged from multiple space-weather sources; retain each contributing dataset's attribution"
            }
        }
    }

    /// Reference [`Provenance`] record for this source format.
    pub const fn provenance(self) -> Provenance {
        match self {
            Self::Gfz => super::gfz::GFZ_PROVENANCE,
            Self::NoaaDailySolarData => super::noaa::NOAA_DSD_PROVENANCE,
            Self::NoaaF107Json => super::noaa::NOAA_F107_PROVENANCE,
            Self::CelestrakCsv => super::celestrak::CELESTRAK_CSV_PROVENANCE,
            Self::CelestrakLegacy => super::celestrak::CELESTRAK_LEGACY_PROVENANCE,
            Self::Merged => super::gfz::GFZ_PROVENANCE,
        }
    }
}

/// Quality or forecast status of a daily space-weather record, ordered from
/// lowest to highest precedence when merging overlapping dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexQuality {
    /// Monthly-stepped long-range prediction (lacks daily `ap` indices).
    MonthlyPredicted,
    /// Daily prediction (such as 45-day solar and geomagnetic forecasts).
    Predicted,
    /// Missing measurement filled by interpolation.
    Interpolated,
    /// Near-real-time, nowcast, or preliminary observation.
    Provisional,
    /// Finalized / definitive observed values.
    Definitive,
}

/// One UTC calendar day of geomagnetic and 10.7 cm solar radio flux indices.
///
/// Sunspot numbers (`SN` / `ISN`) are intentionally omitted so CC BY 4.0 and
/// public-domain atmospheric consumers never ingest CC BY-NC 4.0 sunspot
/// data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceWeatherDay {
    /// Calendar date (UTC).
    pub date: CalendarDate,
    /// Three-hourly planetary equivalent amplitude `ap` for UT intervals
    /// `00–03`, `03–06`, ..., `21–24`.
    pub ap: [Option<f64>; 8],
    /// Daily equivalent planetary amplitude `Ap`.
    pub daily_ap: Option<f64>,
    /// Observed 10.7 cm solar radio flux at actual Earth–Sun distance (s.f.u.).
    pub f107_observed: Option<f64>,
    /// 10.7 cm solar radio flux adjusted to 1 AU (s.f.u.).
    pub f107_adjusted: Option<f64>,
    /// Centered 81-day mean of observed F10.7 (s.f.u.), when supplied by the file.
    pub f107_observed_center81: Option<f64>,
    /// Centered 81-day mean of adjusted F10.7 (s.f.u.), when supplied by the file.
    pub f107_adjusted_center81: Option<f64>,
    /// Quality or forecast tier of this record.
    pub quality: IndexQuality,
    /// Source format this record originated from.
    pub source: SpaceWeatherSource,
}

impl SpaceWeatherDay {
    /// Calendar year.
    pub const fn year(&self) -> i32 {
        self.date.year
    }

    /// Calendar month (`1..=12`).
    pub const fn month(&self) -> u8 {
        self.date.month
    }

    /// Calendar day of the month (`1..=31`).
    pub const fn day(&self) -> u8 {
        self.date.day
    }

    /// 1-based day of the year (`1..=366`).
    pub fn day_of_year(&self) -> u16 {
        self.date.day_of_year()
    }
}

/// Resolved observed solar flux inputs (`F10.7` previous day and 81-day mean)
/// in solar flux units (`10^-22 W m^-2 Hz^-1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarFluxInputs {
    /// Observed F10.7 for the UTC day prior to the target date (s.f.u.).
    pub previous_day: f64,
    /// 81-day mean of observed F10.7 centered on the target date (s.f.u.).
    pub mean_81_day: f64,
}

/// Complete NRLMSISE-00 solar and geomagnetic activity inputs for a UTC instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceWeatherActivity {
    /// 1-based day of the year (`1..=366`).
    pub day_of_year: u16,
    /// Seconds within the UTC day (`0.0..86_400.0`).
    pub ut_seconds: f64,
    /// Observed F10.7 of the previous UTC day (s.f.u.).
    pub f107_previous_day: f64,
    /// Centered 81-day mean of observed F10.7 (s.f.u.).
    pub f107_81_day_mean: f64,
    /// Geomagnetic activity (`History` or `DailyAp`).
    pub geomagnetic: GeomagneticActivity,
}

impl SpaceWeatherActivity {
    /// Combines this activity snapshot with `position` into an [`MsisInput`],
    /// computing apparent local solar time from `ut_seconds` and longitude.
    pub fn into_msis_input(self, position: GeodeticPosition) -> MsisInput {
        MsisInput {
            position,
            day_of_year: self.day_of_year,
            ut_seconds: self.ut_seconds,
            local_solar_time_h: MsisInput::apparent_local_solar_time_h(
                self.ut_seconds,
                position.longitude_deg,
            ),
            f107_previous_day: self.f107_previous_day,
            f107_81_day_mean: self.f107_81_day_mean,
            geomagnetic: self.geomagnetic,
        }
    }
}

pub(crate) fn validate_day(day: &SpaceWeatherDay) -> Result<(), SpaceWeatherError> {
    let _ = CalendarDate::new(day.date.year, day.date.month, day.date.day)?;
    for &slot in &day.ap {
        if let Some(v) = slot {
            if !v.is_finite() || v < 0.0 {
                return Err(SpaceWeatherError::at_query(
                    SpaceWeatherErrorKind::InvalidValue {
                        field: "ap",
                        value: v,
                    },
                ));
            }
        }
    }
    if let Some(v) = day.daily_ap {
        if !v.is_finite() || v < 0.0 {
            return Err(SpaceWeatherError::at_query(
                SpaceWeatherErrorKind::InvalidValue {
                    field: "daily_ap",
                    value: v,
                },
            ));
        }
    }
    let fluxes = [
        ("f107_observed", day.f107_observed),
        ("f107_adjusted", day.f107_adjusted),
        ("f107_observed_center81", day.f107_observed_center81),
        ("f107_adjusted_center81", day.f107_adjusted_center81),
    ];
    for (field, maybe) in fluxes {
        if let Some(v) = maybe {
            if !v.is_finite() || v <= 0.0 {
                return Err(SpaceWeatherError::at_query(
                    SpaceWeatherErrorKind::InvalidValue { field, value: v },
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn merge_day(existing: SpaceWeatherDay, incoming: SpaceWeatherDay) -> SpaceWeatherDay {
    let (primary, secondary) = if incoming.quality >= existing.quality {
        (incoming, existing)
    } else {
        (existing, incoming)
    };
    let mut ap = primary.ap;
    let mut used_secondary = false;
    for (dst, &src) in ap.iter_mut().zip(secondary.ap.iter()) {
        if dst.is_none() && src.is_some() {
            *dst = src;
            used_secondary = true;
        }
    }
    let daily_ap = pick_opt(primary.daily_ap, secondary.daily_ap, &mut used_secondary);
    let f107_observed = pick_opt(
        primary.f107_observed,
        secondary.f107_observed,
        &mut used_secondary,
    );
    let f107_adjusted = pick_opt(
        primary.f107_adjusted,
        secondary.f107_adjusted,
        &mut used_secondary,
    );
    let f107_observed_center81 = pick_opt(
        primary.f107_observed_center81,
        secondary.f107_observed_center81,
        &mut used_secondary,
    );
    let f107_adjusted_center81 = pick_opt(
        primary.f107_adjusted_center81,
        secondary.f107_adjusted_center81,
        &mut used_secondary,
    );
    let source = if used_secondary && primary.source != secondary.source {
        SpaceWeatherSource::Merged
    } else {
        primary.source
    };
    SpaceWeatherDay {
        date: primary.date,
        ap,
        daily_ap,
        f107_observed,
        f107_adjusted,
        f107_observed_center81,
        f107_adjusted_center81,
        quality: primary.quality,
        source,
    }
}

fn pick_opt(
    primary: Option<f64>,
    secondary: Option<f64>,
    used_secondary: &mut bool,
) -> Option<f64> {
    match primary {
        Some(v) => Some(v),
        None => {
            if secondary.is_some() {
                *used_secondary = true;
            }
            secondary
        }
    }
}
