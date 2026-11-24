//! P.526-16 §4.5 and §3.2 against published validation results.
//!
//! P.526-16 publishes no validation examples of its own. P.452-18
//! computes its diffraction loss with the same delta-Bullington method
//! (P.452's delta-Bullington section is P.526 §4.5.2 with ε = 22,
//! σ = 0.003 S/m over land), and the P.452 validation examples list that
//! loss as `Ld50` and the smooth-Earth term as `Ldsph`.
//!
//! Source: the "proposed set" of validation profiles and results in the
//! Python port of the WP 3M-approved P.452 reference implementation,
//! github.com/eeveetza/Py452 at
//! commit 77aa8164b2b046850b10ec159428650614c5b16b,
//! `tests/validation_examples/results/test_result_flat_land_{5,100}km.csv`
//! (rows with p = 50 %; profiles `test_profile_flat_land_{5,100}km.csv`
//! are all-zero heights at 10 m and 1 km spacing, regenerated here).
//!
//! The reference code takes λ = 0.2998 / f(GHz) where this crate uses the
//! exact speed of light; the 1.3e-5 relative difference in λ moves the
//! line-of-sight losses by less than 1e-3 dB, hence the tolerance.

use world_propagation::p526::{DeltaBullington, Ground, ProfilePath, SmoothEarth, TerminalPath};
use world_propagation::{Frequency, Loss, PathLoss, Polarisation, ProfilePoint, Signal};

const TOLERANCE_DB: f64 = 1.0e-3;

#[cfg(test)]
fn flat_profile(length_m: f64, step_m: f64) -> Vec<ProfilePoint> {
    let samples = (length_m / step_m).round() as usize;
    (0..=samples)
        .map(|i| ProfilePoint {
            distance_m: i as f64 * step_m,
            height_m: 0.0,
        })
        .collect()
}

#[cfg(test)]
fn vertical(ghz: f64) -> Signal {
    Signal {
        frequency: Frequency::from_ghz(ghz).unwrap(),
        polarisation: Polarisation::Vertical,
    }
}

/// Checks `(f GHz, Ld50 dB)` rows over a flat profile with 10 m antennas;
/// over a flat path L_ba = L_bs, so L = max(L_sph, L_ba) and the
/// reference reports L_dsph = L_d50 for every row used.
#[cfg(test)]
fn check_flat_rows(length_m: f64, step_m: f64, ae_km: f64, rows: &[(f64, f64)]) {
    let profile = flat_profile(length_m, step_m);
    let path = ProfilePath {
        profile: profile.as_slice(),
        tx_antenna_height_m: 10.0,
        rx_antenna_height_m: 10.0,
    };
    let terminals = TerminalPath {
        distance_m: length_m,
        tx_height_m: 10.0,
        rx_height_m: 10.0,
    };
    let delta = DeltaBullington::new(ae_km * 1.0e3, Ground::LAND).unwrap();
    let smooth = SmoothEarth::new(ae_km * 1.0e3, Ground::LAND).unwrap();
    for &(ghz, expected_db) in rows {
        let signal = vertical(ghz);
        let loss = delta.path_loss(&path, &signal).unwrap();
        assert!(
            (loss.loss_db() - expected_db).abs() < TOLERANCE_DB,
            "{ghz} GHz: delta-Bullington {} dB, reference {expected_db} dB",
            loss.loss_db()
        );
        assert!((loss.spherical_earth_db - expected_db).abs() < TOLERANCE_DB);
        assert_eq!(loss.smooth_surface_tx_m, 0.0);
        assert_eq!(loss.smooth_surface_rx_m, 0.0);
        let sphere = smooth.path_loss(&terminals, &signal).unwrap();
        assert!(
            (sphere.loss_db() - expected_db).abs() < TOLERANCE_DB,
            "{ghz} GHz: smooth Earth {} dB, reference {expected_db} dB",
            sphere.loss_db()
        );
    }
}

/// 100 km is beyond the 26.4 km marginal line of sight: the first-term
/// residue formula of §3.1.1 alone, across 0.1 to 50 GHz.
#[test]
fn flat_100_km_trans_horizon_matches_p452_validation() {
    check_flat_rows(
        100_000.0,
        1_000.0,
        8735.511968,
        &[
            (0.1, 67.30014953),
            (0.25, 70.36696319),
            (0.5, 76.11124705),
            (1.0, 85.58724637),
            (2.0, 99.60559847),
            (5.0, 126.50485015),
            (10.0, 154.17401718),
            (20.0, 189.95926961),
            (50.0, 253.50188255),
        ],
    );
}

/// 5 km is inside line of sight: the §3.2 interpolation, vertically
/// polarised so β and K matter below 300 MHz, and zero loss once the
/// path clears 0.552 of the first Fresnel zone (2 GHz and above).
#[test]
fn flat_5_km_line_of_sight_matches_p452_validation() {
    check_flat_rows(
        5_000.0,
        10.0,
        8738.167287,
        &[
            (0.1, 20.02432081),
            (0.25, 12.69951778),
            (0.5, 7.30775532),
            (0.75, 4.11082622),
            (1.0, 1.75163653),
            (2.0, 0.0),
            (10.0, 0.0),
        ],
    );
}

/// The validation's `land_70km` profile exercises the smooth-surface fit
/// of eqs (58) to (64) and a trans-horizon Bullington point. The profile
/// is ITU validation data whose redistribution terms are not stated, so
/// it is not committed: point `P452_LAND_70KM_PROFILE` at
/// `test_profile_land_70km.csv` from the same commit and run with
/// `--ignored`.
#[test]
#[ignore = "needs the P.452 validation profile file named by P452_LAND_70KM_PROFILE"]
fn land_70_km_matches_p452_validation() {
    let path = std::env::var("P452_LAND_70KM_PROFILE").unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let profile: Vec<ProfilePoint> = text
        .lines()
        .skip(1)
        .map(|line| {
            let mut fields = line.split(',');
            let d_km: f64 = fields.next().unwrap().trim().parse().unwrap();
            let h_m: f64 = fields.next().unwrap().trim().parse().unwrap();
            ProfilePoint {
                distance_m: d_km * 1.0e3,
                height_m: h_m,
            }
        })
        .collect();
    let path = ProfilePath {
        profile: profile.as_slice(),
        tx_antenna_height_m: 10.0,
        rx_antenna_height_m: 10.0,
    };
    let signal = Signal {
        frequency: Frequency::from_ghz(2.0).unwrap(),
        polarisation: Polarisation::Horizontal,
    };
    let model = DeltaBullington::new(9022.617660e3, Ground::LAND).unwrap();
    let loss = model.path_loss(&path, &signal).unwrap();
    assert!((loss.smooth_surface_tx_m - 806.386719).abs() < 1e-5);
    assert!((loss.smooth_surface_rx_m - 673.064055).abs() < 1e-5);
    assert!((loss.spherical_earth_db - 40.65508633).abs() < TOLERANCE_DB);
    // L_ba and L_bs are not in the reference file; these are this
    // crate's values, pinned once L_d50 and L_dsph matched.
    assert!((loss.actual_bullington_db - 44.753_813_7).abs() < 1e-6);
    assert!((loss.smooth_bullington_db - 26.054_623_2).abs() < 1e-6);
    assert!(
        (loss.loss_db - 59.35426906).abs() < TOLERANCE_DB,
        "{loss:?}"
    );
}
