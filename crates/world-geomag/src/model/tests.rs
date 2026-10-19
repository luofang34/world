//! Evaluator domain, pole and uncertainty tests; the published test values
//! are in `tests/wmm2025_test_values.rs`.

use super::*;
use crate::GeomagError;

fn surface(latitude_deg: f64, longitude_deg: f64) -> GeodeticPosition {
    GeodeticPosition::wgs84(latitude_deg, longitude_deg, 0.0)
}

#[test]
fn dates_outside_the_validity_window_are_refused() {
    for year in [2024.99, 2030.01, f64::NAN] {
        assert!(matches!(
            wmm2025(surface(47.4, 8.5), DecimalYear(year)),
            Err(GeomagError::OutsideValidity { .. })
        ));
    }
    assert!(wmm2025(surface(47.4, 8.5), DecimalYear(2025.0)).is_ok());
    assert!(wmm2025(surface(47.4, 8.5), DecimalYear(2030.0)).is_ok());
}

#[test]
fn positions_outside_the_domain_are_refused() {
    let bad = [
        surface(90.5, 0.0),
        surface(f64::NAN, 0.0),
        surface(0.0, f64::INFINITY),
        GeodeticPosition::wgs84(0.0, 0.0, 851_000.0),
        GeodeticPosition::wgs84(0.0, 0.0, -1_001.0),
        GeodeticPosition::wgs84(0.0, 0.0, f64::NAN),
    ];
    for p in bad {
        assert_eq!(
            wmm2025(p, DecimalYear(2026.0)),
            Err(GeomagError::InvalidPosition),
            "{p:?}"
        );
    }
}

#[test]
fn the_geographic_poles_give_a_finite_field() {
    for lat in [90.0, -90.0] {
        let field = wmm2025(surface(lat, 0.0), DecimalYear(2027.5)).expect("inside the domain");
        for v in [
            field.north_nt,
            field.east_nt,
            field.down_nt,
            field.declination_deg,
            field.uncertainty.declination_deg.one_sigma,
        ] {
            assert!(v.is_finite(), "lat {lat}: {field:?}");
        }
        // Near the north pole the field points steeply down, near the
        // south pole steeply up.
        assert!(field.inclination_deg.abs() > 70.0);
    }
}

#[test]
fn declination_uncertainty_grows_where_the_horizontal_field_is_weak() {
    let date = DecimalYear(2026.0);
    let mid_latitude = wmm2025(surface(40.6, -74.7), date).expect("inside the domain");
    let near_pole = wmm2025(surface(85.0, -120.0), date).expect("inside the domain");
    assert!(near_pole.horizontal_nt < mid_latitude.horizontal_nt);
    let sigma = |field: &MagneticField| field.uncertainty.declination_deg.one_sigma;
    assert!(sigma(&near_pole) > sigma(&mid_latitude));
    // Where the horizontal field is strong the declination term
    // approaches its 0.26 degree floor.
    let h = mid_latitude.horizontal_nt;
    let expected = libm::sqrt(0.26 * 0.26 + (5_417.0 / h) * (5_417.0 / h));
    assert_eq!(sigma(&mid_latitude), expected);
    assert!(sigma(&mid_latitude) > 0.26);
    assert_eq!(mid_latitude.uncertainty.north_nt.one_sigma, 137.0);
}
