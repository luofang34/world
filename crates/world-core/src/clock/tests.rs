use crate::{ClockCorrespondence, ClockError, WorldInstant};

const BOOT_UTC_MICROS: i64 = 1_767_225_600_000_000;

#[test]
fn a_correspondence_maps_both_ways_through_the_same_instant() {
    let clocks = ClockCorrespondence::new(5_000_000, BOOT_UTC_MICROS);
    let later = clocks.at_monotonic(65_000_000).unwrap();
    assert_eq!(later.utc_unix_micros, BOOT_UTC_MICROS + 60_000_000);
    assert_eq!(clocks.at_utc(later.utc_unix_micros).unwrap(), later);
}

#[test]
fn a_utc_time_before_the_monotonic_origin_is_refused() {
    let clocks = ClockCorrespondence::new(5_000_000, BOOT_UTC_MICROS);
    let before_origin = BOOT_UTC_MICROS - 5_000_001;
    assert_eq!(
        clocks.at_utc(before_origin),
        Err(ClockError::OutsideMonotonicClock {
            utc_unix_micros: before_origin
        })
    );
}

#[test]
fn a_utc_value_past_its_range_is_refused() {
    let clocks = ClockCorrespondence::new(0, i64::MAX);
    assert_eq!(
        clocks.at_monotonic(1),
        Err(ClockError::UtcOutOfRange {
            monotonic_micros: 1
        })
    );
}

#[test]
fn an_instant_gives_the_decimal_year_of_its_utc() {
    // 2026-01-01T00:00:00Z plus half a second.
    let instant = WorldInstant::new(0, BOOT_UTC_MICROS + 500_000);
    let year = instant.decimal_year().value();
    assert!(year > 2026.0 && year < 2026.0 + 1.0 / (365.0 * 86_400.0));
}
