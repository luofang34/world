use super::*;
use crate::workspace_len;

const A: f64 = 6_371_200.0;

fn window() -> ValidityWindow {
    ValidityWindow {
        from: DecimalYear(2020.0),
        until: DecimalYear(2030.0),
    }
}

fn field<B: AsRef<[f64]>>(model: &MagneticModel<B>, p: [f64; 3], year: f64) -> [f64; 3] {
    let mut buffer = [0.0; workspace_len(1)];
    let mut ws = Workspace::new(&mut buffer, 1).unwrap();
    model
        .field(&mut ws, p, DecimalYear(year), Derivatives::Gradient)
        .unwrap()
        .vector
}

#[test]
fn an_axial_dipole_points_north_at_the_equator_and_in_at_the_pole() {
    let b0 = 30_000.0;
    let model =
        MagneticModel::tilted_dipole(A, b0, 90.0, 0.0, DecimalYear(2025.0), window()).unwrap();
    // The dipole field is B0 (a/r)^3 at the equator and twice it at the
    // poles; agreement is rounding (1e-10 of B0 is ~3 µnT), and the zero
    // components are rounding of 30 000 nT terms (1e-9 nT absolute).
    let eq = field(&model, [A, 0.0, 0.0], 2025.0);
    assert!(eq[0].abs() < 1e-9 && eq[1].abs() < 1e-9);
    assert!((eq[2] - b0).abs() < 1e-10 * b0, "{eq:?}");
    let far = field(&model, [0.0, 2.0 * A, 0.0], 2025.0);
    assert!((far[2] - b0 / 8.0).abs() < 1e-10 * b0);
    let pole = field(&model, [0.0, 0.0, A], 2025.0);
    assert!((pole[2] + 2.0 * b0).abs() < 1e-10 * b0, "{pole:?}");
}

#[test]
fn a_tilted_dipole_points_into_the_body_at_its_pole() {
    let (b0, lat, lon) = (29_400.0_f64, 80.7_f64, -72.7_f64);
    let model =
        MagneticModel::tilted_dipole(A, b0, lat, lon, DecimalYear(2025.0), window()).unwrap();
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    let up = [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()];
    let b = field(&model, up.map(|u| u * A), 2025.0);
    // The pole position goes through sin/cos and back: 1e-9 of B0.
    for i in 0..3 {
        assert!((b[i] + 2.0 * b0 * up[i]).abs() < 1e-9 * b0, "{b:?}");
    }
}

#[test]
fn secular_variation_is_linear_in_the_date() {
    let main = HarmonicCoefficients::new(1, [0.0, -17_000.0, -800.0], [0.0, 0.0, 2_600.0]).unwrap();
    let rate = HarmonicCoefficients::new(1, [0.0, 7.0, 5.0], [0.0, 0.0, -12.0]).unwrap();
    let model = MagneticModel::new(A, DecimalYear(2020.0), window(), main, rate).unwrap();
    let p = [3.0e6, -4.0e6, 4.2e6];
    let (b0, b4, b8) = (
        field(&model, p, 2020.0),
        field(&model, p, 2024.0),
        field(&model, p, 2028.0),
    );
    // Linear in date by construction; 1e-9 relative is rounding of the
    // dated coefficients.
    for i in 0..3 {
        assert!((b4[i] - 0.5 * (b0[i] + b8[i])).abs() < 1e-9 * b0[i].abs().max(1.0));
    }
}

#[test]
fn dates_outside_the_window_are_refused_not_extrapolated() {
    let model =
        MagneticModel::dipole(A, [-29_000.0, 0.0, 0.0], DecimalYear(2025.0), window()).unwrap();
    let mut buffer = [0.0; workspace_len(1)];
    let mut ws = Workspace::new(&mut buffer, 1).unwrap();
    for year in [2019.99, 2030.01, f64::NAN] {
        assert!(matches!(
            model.field(
                &mut ws,
                [A, 0.0, 0.0],
                DecimalYear(year),
                Derivatives::Gradient
            ),
            Err(FieldError::OutsideValidity { .. })
        ));
    }
}

#[test]
fn schmidt_coefficients_scale_by_root_two_n_plus_one() {
    // One division by a square root: within an ulp or two.
    assert_eq!(schmidt_to_geodesy(0, 2.0), 2.0);
    assert!((schmidt_to_geodesy(1, 3.0_f64.sqrt()) - 1.0).abs() < 1e-16);
    assert!((schmidt_to_geodesy(12, 5.0) - 1.0).abs() < 1e-15);
}
