//! Chronological daily space-weather lookup table and NRLMSISE-00 synthesis.

use alloc::vec::Vec;

use world_core::{GeodeticPosition, WorldInstant};

use crate::{ApHistory, GeomagneticActivity, MsisInput, math};

use super::calendar::CalendarDate;
use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use super::record::{
    SolarFluxInputs, SpaceWeatherActivity, SpaceWeatherDay, merge_day, validate_day,
};

const SECONDS_PER_BUCKET: f64 = 10_800.0;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Chronologically ordered daily space-weather lookup table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpaceWeatherTable {
    days: Vec<SpaceWeatherDay>,
}

impl SpaceWeatherTable {
    /// An empty lookup table.
    pub const fn new() -> Self {
        Self { days: Vec::new() }
    }

    /// Builds a sorted, deduplicated lookup table from daily records.
    ///
    /// Duplicate dates are merged by [`super::IndexQuality`] precedence (higher
    /// quality wins per field, filling `None` gaps from the lower-precedence
    /// record; ties favor later records).
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidDate`] or
    /// [`SpaceWeatherErrorKind::InvalidValue`] if any record contains an invalid
    /// calendar date, non-finite index, negative `ap`/`Ap`, or non-positive
    /// `F10.7`.
    pub fn from_days<I>(days: I) -> Result<Self, SpaceWeatherError>
    where
        I: IntoIterator<Item = SpaceWeatherDay>,
    {
        let mut collected: Vec<SpaceWeatherDay> = Vec::new();
        for day in days {
            validate_day(&day)?;
            collected.push(day);
        }
        collected.sort_by_key(|d| d.date);
        Ok(Self {
            days: dedup_sorted(collected),
        })
    }

    /// Number of daily records stored in the table.
    pub fn len(&self) -> usize {
        self.days.len()
    }

    /// `true` when the table contains no records.
    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// Inclusive `(earliest, latest)` calendar date span, or `None` if empty.
    pub fn date_range(&self) -> Option<(CalendarDate, CalendarDate)> {
        let first = self.days.first()?.date;
        let last = self.days.last()?.date;
        Some((first, last))
    }

    /// Looks up the record for `date` in `O(log n)` time.
    pub fn get(&self, date: CalendarDate) -> Option<&SpaceWeatherDay> {
        let idx = self.days.binary_search_by_key(&date, |d| d.date).ok()?;
        self.days.get(idx)
    }

    /// Chronologically ordered slice of all daily records.
    pub fn days(&self) -> &[SpaceWeatherDay] {
        &self.days
    }

    /// Consumes the table and returns the chronologically ordered records.
    pub fn into_days(self) -> Vec<SpaceWeatherDay> {
        self.days
    }

    /// Merges records from `other` into `self` in chronological order.
    ///
    /// Overlapping dates combine according to [`super::IndexQuality`]
    /// precedence: higher-quality values win and `None` slots are filled from
    /// the secondary record; equal quality favors `other`.
    pub fn merge(&mut self, other: &Self) {
        let mut merged = Vec::with_capacity(self.days.len() + other.days.len());
        let mut i = 0usize;
        let mut j = 0usize;
        while let (Some(a), Some(b)) = (self.days.get(i), other.days.get(j)) {
            if a.date < b.date {
                merged.push(*a);
                i = i.wrapping_add(1);
            } else if b.date < a.date {
                merged.push(*b);
                j = j.wrapping_add(1);
            } else {
                merged.push(merge_day(*a, *b));
                i = i.wrapping_add(1);
                j = j.wrapping_add(1);
            }
        }
        while let Some(a) = self.days.get(i) {
            merged.push(*a);
            i = i.wrapping_add(1);
        }
        while let Some(b) = other.days.get(j) {
            merged.push(*b);
            j = j.wrapping_add(1);
        }
        self.days = merged;
    }

    /// Synthesizes the exact 3-hourly [`ApHistory`] at `(date, ut_seconds)`,
    /// stepping seamlessly across UTC midnight boundaries.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidTime`] if `ut_seconds` is not in
    /// `[0.0, 86_400.0)`, [`SpaceWeatherErrorKind::MissingDate`] if any day in
    /// the 57-hour lookback window is absent, or
    /// [`SpaceWeatherErrorKind::MissingIndex`] if `daily_ap` or any required
    /// 3-hourly `ap` slot is missing.
    pub fn ap_history_at(
        &self,
        date: CalendarDate,
        ut_seconds: f64,
    ) -> Result<ApHistory, SpaceWeatherError> {
        let bucket = ut_bucket(ut_seconds)?;
        let target = self.require_day(date)?;
        let daily = target.daily_ap.ok_or(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::MissingIndex {
                date,
                index: "daily_ap",
            },
        ))?;
        let slot = date.days_since_unix_epoch() * 8 + bucket;
        Ok(ApHistory {
            daily,
            current: self.ap_at_slot(slot)?,
            hours_3_before: self.ap_at_slot(slot - 1)?,
            hours_6_before: self.ap_at_slot(slot - 2)?,
            hours_9_before: self.ap_at_slot(slot - 3)?,
            mean_12_to_33_hours_before: self.ap_mean(slot - 11, 8)?,
            mean_36_to_57_hours_before: self.ap_mean(slot - 19, 8)?,
        })
    }

    /// Resolves observed `F10.7` of the previous day and the 81-day centered
    /// mean for `date`.
    ///
    /// Uses `f107_observed_center81` when present on `date`'s record, or
    /// computes the arithmetic mean of `f107_observed` across the 81
    /// consecutive calendar days `date - 40 ..= date + 40`.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::MissingDate`] or
    /// [`SpaceWeatherErrorKind::MissingIndex`] if the target day, previous day,
    /// or any day in the 81-day window lacks observed F10.7.
    pub fn f107_at(&self, date: CalendarDate) -> Result<SolarFluxInputs, SpaceWeatherError> {
        let target = self.require_day(date)?;
        let prev_date = date.add_days(-1)?;
        let prev = self.require_day(prev_date)?;
        let previous_day = prev.f107_observed.ok_or(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::MissingIndex {
                date: prev_date,
                index: "f107_observed",
            },
        ))?;
        let mean_81_day = match target.f107_observed_center81 {
            Some(mean) => mean,
            None => self.centered_81_day_mean(date)?,
        };
        Ok(SolarFluxInputs {
            previous_day,
            mean_81_day,
        })
    }

    /// Synthesizes full NRLMSISE-00 activity indices (with 3-hourly
    /// [`ApHistory`]) at `(date, ut_seconds)`.
    ///
    /// # Errors
    ///
    /// As [`Self::f107_at`] and [`Self::ap_history_at`].
    pub fn activity_at(
        &self,
        date: CalendarDate,
        ut_seconds: f64,
    ) -> Result<SpaceWeatherActivity, SpaceWeatherError> {
        let ap_history = self.ap_history_at(date, ut_seconds)?;
        let flux = self.f107_at(date)?;
        Ok(SpaceWeatherActivity {
            day_of_year: date.day_of_year(),
            ut_seconds,
            f107_previous_day: flux.previous_day,
            f107_81_day_mean: flux.mean_81_day,
            geomagnetic: GeomagneticActivity::History(ap_history),
        })
    }

    /// Synthesizes NRLMSISE-00 activity indices using daily `Ap` only at
    /// `(date, ut_seconds)`.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherErrorKind::InvalidTime`], [`Self::f107_at`] errors, or
    /// [`SpaceWeatherErrorKind::MissingIndex`] if `daily_ap` is missing.
    pub fn activity_daily_ap_at(
        &self,
        date: CalendarDate,
        ut_seconds: f64,
    ) -> Result<SpaceWeatherActivity, SpaceWeatherError> {
        let _bucket = ut_bucket(ut_seconds)?;
        let target = self.require_day(date)?;
        let daily = target.daily_ap.ok_or(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::MissingIndex {
                date,
                index: "daily_ap",
            },
        ))?;
        let flux = self.f107_at(date)?;
        Ok(SpaceWeatherActivity {
            day_of_year: date.day_of_year(),
            ut_seconds,
            f107_previous_day: flux.previous_day,
            f107_81_day_mean: flux.mean_81_day,
            geomagnetic: GeomagneticActivity::DailyAp(daily),
        })
    }

    /// Builds and validates a complete [`MsisInput`] for `position` at
    /// `(date, ut_seconds)` using the 3-hourly [`ApHistory`] formulation.
    ///
    /// # Errors
    ///
    /// [`SpaceWeatherError`] if required indices are absent or the resulting
    /// [`MsisInput`] fails validation (such as invalid latitude).
    pub fn msis_input_at(
        &self,
        position: GeodeticPosition,
        date: CalendarDate,
        ut_seconds: f64,
    ) -> Result<MsisInput, SpaceWeatherError> {
        let input = self
            .activity_at(date, ut_seconds)?
            .into_msis_input(position);
        input.validate()?;
        Ok(input)
    }

    /// Builds and validates a complete [`MsisInput`] for `position` at a
    /// [`WorldInstant`], preserving sub-second UTC offsets.
    ///
    /// # Errors
    ///
    /// As [`Self::msis_input_at`].
    pub fn msis_input_at_instant(
        &self,
        position: GeodeticPosition,
        instant: WorldInstant,
    ) -> Result<MsisInput, SpaceWeatherError> {
        let whole_secs = instant.utc_unix_micros.div_euclid(1_000_000);
        let sub_micros = instant.utc_unix_micros.rem_euclid(1_000_000);
        let (date, sec_of_day) = CalendarDate::from_unix_seconds(whole_secs)?;
        let ut_seconds = sec_of_day + (sub_micros as f64) / 1_000_000.0;
        self.msis_input_at(position, date, ut_seconds)
    }

    fn require_day(&self, date: CalendarDate) -> Result<&SpaceWeatherDay, SpaceWeatherError> {
        self.get(date).ok_or(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::MissingDate { date },
        ))
    }

    fn ap_at_slot(&self, slot: i64) -> Result<f64, SpaceWeatherError> {
        let date = CalendarDate::from_days_since_unix_epoch(slot.div_euclid(8))?;
        let bucket = usize::try_from(slot.rem_euclid(8)).unwrap_or(0);
        let record = self.require_day(date)?;
        record
            .ap
            .get(bucket)
            .copied()
            .flatten()
            .ok_or(SpaceWeatherError::at_query(
                SpaceWeatherErrorKind::MissingIndex { date, index: "ap" },
            ))
    }

    fn ap_mean(&self, start_slot: i64, count: i64) -> Result<f64, SpaceWeatherError> {
        let mut sum = 0.0;
        for offset in 0..count {
            sum += self.ap_at_slot(start_slot + offset)?;
        }
        Ok(sum / (count as f64))
    }

    fn centered_81_day_mean(&self, center: CalendarDate) -> Result<f64, SpaceWeatherError> {
        let mut sum = 0.0;
        for offset in -40_i64..=40_i64 {
            let date = center.add_days(offset)?;
            let day = self.require_day(date)?;
            let flux = day.f107_observed.ok_or(SpaceWeatherError::at_query(
                SpaceWeatherErrorKind::MissingIndex {
                    date,
                    index: "f107_observed",
                },
            ))?;
            sum += flux;
        }
        Ok(sum / 81.0)
    }
}

fn ut_bucket(ut_seconds: f64) -> Result<i64, SpaceWeatherError> {
    if !ut_seconds.is_finite() || !(0.0..SECONDS_PER_DAY).contains(&ut_seconds) {
        return Err(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::InvalidTime { ut_seconds },
        ));
    }
    let raw = math::floor(ut_seconds / SECONDS_PER_BUCKET) as i64;
    Ok(raw.clamp(0, 7))
}

fn dedup_sorted(days: Vec<SpaceWeatherDay>) -> Vec<SpaceWeatherDay> {
    let mut out: Vec<SpaceWeatherDay> = Vec::with_capacity(days.len());
    for day in days {
        if let Some(last) = out.last_mut() {
            if last.date == day.date {
                *last = merge_day(*last, day);
                continue;
            }
        }
        out.push(day);
    }
    out
}
