//! The §4.5.1 Bullington construction and the §4.5.2 smooth-surface fit
//! on hand-built profiles, where the Bullington point and the fitted
//! surface follow from the geometry by hand.

use world_propagation::p526::{
    DeltaBullington, Ground, ProfilePath, diffraction_parameter, knife_edge_loss_approx_db,
};
use world_propagation::{Frequency, PathLoss, Polarisation, ProfilePoint, Signal};

const AE_M: f64 = 8_500_000.0;

#[cfg(test)]
fn profile(points_km_m: &[(f64, f64)]) -> Vec<ProfilePoint> {
    points_km_m
        .iter()
        .map(|&(d_km, h_m)| ProfilePoint {
            distance_m: d_km * 1.0e3,
            height_m: h_m,
        })
        .collect()
}

#[cfg(test)]
fn signal() -> Signal {
    Signal {
        frequency: Frequency::from_ghz(1.0).unwrap(),
        polarisation: Polarisation::Horizontal,
    }
}

/// Eq. (57) for an edge of diffraction parameter `nu` on a `d_km` path.
#[cfg(test)]
fn bullington_db(nu: f64, d_km: f64) -> f64 {
    let l_uc = knife_edge_loss_approx_db(nu);
    l_uc + (1.0 - libm::exp(-l_uc / 6.0)) * (10.0 + 0.02 * d_km)
}

/// Earth bulge d₁d₂ / 2aₑ at a point, m.
#[cfg(test)]
fn bulge_m(d1_m: f64, d2_m: f64) -> f64 {
    d1_m * d2_m / (2.0 * AE_M)
}

#[cfg(test)]
fn actual_bullington(points: &[(f64, f64)], antenna_m: f64) -> f64 {
    let terrain = profile(points);
    let path = ProfilePath {
        profile: terrain.as_slice(),
        tx_antenna_height_m: antenna_m,
        rx_antenna_height_m: antenna_m,
    };
    let model = DeltaBullington::new(AE_M, Ground::LAND).unwrap();
    model
        .path_loss(&path, &signal())
        .unwrap()
        .actual_bullington_db
}

#[test]
fn single_blocking_peak_is_the_bullington_point() {
    // Both horizon rays graze the peak, so it is the knife edge.
    let lambda = signal().frequency.wavelength_m();
    let edge_m = 200.0 + bulge_m(10.0e3, 10.0e3) - 10.0;
    let nu = diffraction_parameter(edge_m, 10.0e3, 10.0e3, lambda).unwrap();
    let got = actual_bullington(&[(0.0, 0.0), (10.0, 200.0), (20.0, 0.0)], 10.0);
    assert!((got - bullington_db(nu, 20.0)).abs() < 1e-9, "{got}");
}

#[test]
fn line_of_sight_branch_takes_the_highest_nu() {
    // The peak sits just under the ray (h = −1.1 m with the 5.9 m bulge): eq. (51), with
    // ν > −0.78 so eq. (31) still applies.
    let lambda = signal().frequency.wavelength_m();
    let edge_m = 23.0 + bulge_m(10.0e3, 10.0e3) - 30.0;
    assert!(edge_m < 0.0);
    let nu = diffraction_parameter(edge_m, 10.0e3, 10.0e3, lambda).unwrap();
    assert!(nu > -0.78);
    let got = actual_bullington(&[(0.0, 0.0), (10.0, 23.0), (20.0, 0.0)], 30.0);
    assert!((got - bullington_db(nu, 20.0)).abs() < 1e-9, "{got}");
}

#[test]
fn two_peaks_diffract_at_the_crossing_of_the_horizon_rays() {
    // Each terminal's horizon ray grazes the nearer peak; by symmetry the
    // rays cross mid-path, Stim·10 km above the 10 m chord.
    let lambda = signal().frequency.wavelength_m();
    let slope_m_per_km = (100.0 + bulge_m(5.0e3, 15.0e3) - 10.0) / 5.0;
    let edge_m = slope_m_per_km * 10.0;
    let nu = diffraction_parameter(edge_m, 10.0e3, 10.0e3, lambda).unwrap();
    let got = actual_bullington(
        &[(0.0, 0.0), (5.0, 100.0), (15.0, 100.0), (20.0, 0.0)],
        10.0,
    );
    assert!((got - bullington_db(nu, 20.0)).abs() < 1e-9, "{got}");
}

#[test]
fn smooth_surface_of_a_ramp_is_the_ramp() {
    // A least-squares line through a straight profile is the profile.
    let points: Vec<(f64, f64)> = (0..=20)
        .map(|i| (f64::from(i), 100.0 + 10.0 * f64::from(i)))
        .collect();
    let terrain = profile(&points);
    let path = ProfilePath {
        profile: terrain.as_slice(),
        tx_antenna_height_m: 10.0,
        rx_antenna_height_m: 10.0,
    };
    let model = DeltaBullington::new(AE_M, Ground::LAND).unwrap();
    let loss = model.path_loss(&path, &signal()).unwrap();
    assert!((loss.smooth_surface_tx_m - 100.0).abs() < 1e-9);
    assert!((loss.smooth_surface_rx_m - 300.0).abs() < 1e-9);
}

#[test]
fn obstruction_lowers_the_fitted_surface() {
    // Eqs (58) to (60) give v₁ = 2200, v₂ = 66000 and h_stip = h_srip
    // = 55 m. The 120 m bump stands h_obs = 10 m above the 110 m chord
    // with α_obt = α_obr = 1, so eqs (62c) to (62f) take 5 m off each end.
    let terrain = profile(&[
        (0.0, 100.0),
        (5.0, 0.0),
        (10.0, 120.0),
        (15.0, 0.0),
        (20.0, 100.0),
    ]);
    let path = ProfilePath {
        profile: terrain.as_slice(),
        tx_antenna_height_m: 10.0,
        rx_antenna_height_m: 10.0,
    };
    let model = DeltaBullington::new(AE_M, Ground::LAND).unwrap();
    let loss = model.path_loss(&path, &signal()).unwrap();
    assert!((loss.smooth_surface_tx_m - 50.0).abs() < 1e-9, "{loss:?}");
    assert!((loss.smooth_surface_rx_m - 50.0).abs() < 1e-9);
}

#[test]
fn short_or_unordered_profiles_are_refused() {
    let model = DeltaBullington::new(AE_M, Ground::LAND).unwrap();
    let two = profile(&[(0.0, 0.0), (10.0, 0.0)]);
    let backwards = profile(&[(0.0, 0.0), (10.0, 0.0), (5.0, 0.0)]);
    for terrain in [two, backwards] {
        let path = ProfilePath {
            profile: terrain.as_slice(),
            tx_antenna_height_m: 10.0,
            rx_antenna_height_m: 10.0,
        };
        assert!(model.path_loss(&path, &signal()).is_err());
    }
}
