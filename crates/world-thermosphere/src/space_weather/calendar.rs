//! Proleptic Gregorian calendar dates, day-of-year conversion, and exact
//! day-step arithmetic.

use core::fmt;

use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};

const SECONDS_PER_DAY_I64: i64 = 86_400;
const MIN_YEAR: i32 = -999_999;
const MAX_YEAR: i32 = 999_999;
const CUMULATIVE_MONTH_DAYS: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

/// Returns `true` when `year` is a leap year in the proleptic Gregorian
/// calendar.
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Computes the 1-based day of the year (`1..=366`) for a calendar date.
///
/// # Errors
///
/// [`SpaceWeatherErrorKind::InvalidDate`] if the calendar date is invalid.
pub fn day_of_year(year: i32, month: u8, day: u8) -> Result<u16, SpaceWeatherError> {
    Ok(CalendarDate::new(year, month, day)?.day_of_year())
}

/// Returns the number of days in `month` (`1..=12`) of `year`, or `None` if
/// `month` is out of range.
const fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        _ => None,
    }
}

/// A UTC calendar date in the proleptic Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    /// Calendar year.
    pub year: i32,
    /// Month of the year (`1..=12`).
    pub month: u8,
    /// Day of the month (`1..=31`).
    pub day: u8,
}

impl CalendarDate {
    /// Validates and constructs a calendar date.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidDate`] when `month`, `day`, or `year`
    /// is outside valid Gregorian bounds.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, SpaceWeatherError> {
        let invalid =
            || SpaceWeatherError::at_query(SpaceWeatherErrorKind::InvalidDate { year, month, day });
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(invalid());
        }
        let max_day = days_in_month(year, month).ok_or_else(invalid)?;
        if !(1..=max_day).contains(&day) {
            return Err(invalid());
        }
        Ok(Self { year, month, day })
    }

    /// Whether this date falls in a leap year.
    pub const fn is_leap_year(self) -> bool {
        is_leap_year(self.year)
    }

    /// The 1-based day of the year (`1..=366`).
    pub fn day_of_year(self) -> u16 {
        let base = CUMULATIVE_MONTH_DAYS
            .get(usize::from(self.month.saturating_sub(1)))
            .copied()
            .unwrap_or(0);
        let leap = u16::from(self.month > 2 && self.is_leap_year());
        base + leap + u16::from(self.day)
    }

    /// Days since `1970-01-01` (Howard Hinnant's civil-from-days conversion).
    pub const fn days_since_unix_epoch(self) -> i64 {
        let m = self.month as i64;
        let d = self.day as i64;
        let y = (self.year as i64) - if m <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Converts whole days since `1970-01-01` back into a [`CalendarDate`].
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidDate`] if the resulting year does not
    /// fit the supported range.
    pub fn from_days_since_unix_epoch(days: i64) -> Result<Self, SpaceWeatherError> {
        let fail = || {
            SpaceWeatherError::at_query(SpaceWeatherErrorKind::InvalidDate {
                year: i32::MAX,
                month: 0,
                day: 0,
            })
        };
        let z = days.checked_add(719_468).ok_or_else(fail)?;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + if mp < 10 { 3 } else { -9 };
        let year_i64 = y + i64::from(m <= 2);
        let year = i32::try_from(year_i64).map_err(|_| fail())?;
        let month = u8::try_from(m).map_err(|_| fail())?;
        let day = u8::try_from(d).map_err(|_| fail())?;
        Self::new(year, month, day)
    }

    /// Shifts this date by `delta` calendar days.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidDate`] on overflow or out-of-range year.
    pub fn add_days(self, delta: i64) -> Result<Self, SpaceWeatherError> {
        let shifted =
            self.days_since_unix_epoch()
                .checked_add(delta)
                .ok_or(SpaceWeatherError::at_query(
                    SpaceWeatherErrorKind::InvalidDate {
                        year: self.year,
                        month: self.month,
                        day: self.day,
                    },
                ))?;
        Self::from_days_since_unix_epoch(shifted)
    }

    /// Converts whole seconds since `1970-01-01T00:00:00Z` into a UTC
    /// [`CalendarDate`] and seconds within that UTC day (`0.0..86_400.0`).
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidDate`] if the year exceeds supported
    /// bounds.
    pub fn from_unix_seconds(unix_seconds: i64) -> Result<(Self, f64), SpaceWeatherError> {
        let days = unix_seconds.div_euclid(SECONDS_PER_DAY_I64);
        let secs = unix_seconds.rem_euclid(SECONDS_PER_DAY_I64) as f64;
        Ok((Self::from_days_since_unix_epoch(days)?, secs))
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
