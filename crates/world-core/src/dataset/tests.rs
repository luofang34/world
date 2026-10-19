use super::*;

#[test]
fn the_window_is_inclusive_and_refuses_nan() {
    let window = ValidityWindow {
        from: DecimalYear(2025.0),
        until: DecimalYear(2030.0),
    };
    assert!(window.contains(DecimalYear(2025.0)));
    assert!(window.contains(DecimalYear(2030.0)));
    for outside in [2024.999, 2030.001, f64::NAN] {
        assert!(!window.contains(DecimalYear(outside)), "{outside}");
    }
}
