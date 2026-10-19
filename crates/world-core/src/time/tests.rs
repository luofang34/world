use super::*;

fn year(unix_seconds: i64) -> f64 {
    DecimalYear::from_unix_seconds(unix_seconds).value()
}

#[test]
fn year_starts_map_to_whole_years() {
    assert_eq!(year(0), 1970.0);
    // 2025-01-01T00:00:00Z
    assert_eq!(year(1_735_689_600), 2025.0);
    // 2030-01-01T00:00:00Z
    assert_eq!(year(1_893_456_000), 2030.0);
}

#[test]
fn a_leap_year_divides_by_its_own_length() {
    // 2028-07-01T00:00:00Z is day 182 of the 366-day year 2028.
    assert!((year(1_846_022_400) - (2028.0 + 182.0 / 366.0)).abs() < 1e-12);
    // The last second of 2027 is still 2027.
    assert!(year(1_830_297_599) < 2028.0);
}

#[test]
fn instants_before_the_unix_epoch_count_back() {
    // 1969-01-01T00:00:00Z
    assert_eq!(year(-31_536_000), 1969.0);
}
