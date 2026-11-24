use super::*;

#[test]
fn frequency_converts_between_units_and_to_wavelength() {
    let f = Frequency::from_ghz(2.0).unwrap();
    assert_eq!(f.hz(), 2.0e9);
    assert!((f.mhz() - 2000.0).abs() < 1e-9);
    assert!((f.ghz() - 2.0).abs() < 1e-15);
    assert!((f.wavelength_m() - 0.149_896_229).abs() < 1e-9);
}

#[test]
fn frequency_refuses_non_positive_and_non_finite() {
    for hz in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Frequency::from_hz(hz).is_err(), "{hz}");
    }
}

#[test]
fn percentage_is_an_open_interval() {
    assert!(Percentage::new(0.0).is_err());
    assert!(Percentage::new(100.0).is_err());
    assert!(Percentage::new(f64::NAN).is_err());
    let p = Percentage::new(50.0).unwrap();
    assert_eq!(p.percent(), 50.0);
    assert_eq!(p.fraction(), 0.5);
}
