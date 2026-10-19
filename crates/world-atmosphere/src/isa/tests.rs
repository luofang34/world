use world_core::DecimalYear;

use super::*;

fn at(geopotential_m: f64) -> AtmosphereState {
    Isa.at(Altitude::Geopotential(geopotential_m))
        .expect("inside the standard")
}

fn relative(got: f64, want: f64) -> f64 {
    ((got - want) / want).abs()
}

/// ISO 2533 / ICAO Doc 7488 tabulated values, geopotential altitude:
/// `(H m, T K, p Pa, ρ kg/m³)`, five significant figures.
const TABLE: [(f64, f64, f64, f64); 11] = [
    (-2_000.0, 301.15, 127_774.0, 1.4781),
    (0.0, 288.15, 101_325.0, 1.2250),
    (1_000.0, 281.65, 89_874.6, 1.1116),
    (5_000.0, 255.65, 54_019.9, 0.73612),
    (11_000.0, 216.65, 22_632.0, 0.36392),
    (20_000.0, 216.65, 5_474.89, 0.088035),
    (32_000.0, 228.65, 868.019, 0.013225),
    (47_000.0, 270.65, 110.906, 0.0014275),
    (51_000.0, 270.65, 66.9389, 0.00086160),
    (71_000.0, 214.65, 3.95642, 0.000064211),
    (80_000.0, 196.65, 0.886_280, 0.000015700),
];

#[test]
fn the_tabulated_standard_atmosphere_is_reproduced() {
    for (h, t, p, rho) in TABLE {
        let state = at(h);
        assert!(
            (state.temperature_k - t).abs() < 1e-9,
            "T at {h}: {state:?}"
        );
        assert!(relative(state.pressure_pa, p) < 2e-5, "p at {h}: {state:?}");
        assert!(
            relative(state.density_kg_m3, rho) < 1e-4,
            "ρ at {h}: {state:?}"
        );
    }
}

#[test]
fn pressure_is_continuous_across_layer_bases() {
    for (base, _, _) in LAYERS.iter().skip(1) {
        let below = at(base - 1e-6).pressure_pa;
        let above = at(base + 1e-6).pressure_pa;
        assert!(relative(above, below) < 1e-9, "{base}");
    }
}

#[test]
fn geometric_altitude_is_converted_before_the_lookup() {
    let geometric = Isa.at(Altitude::GeometricMsl(10_000.0)).expect("inside");
    let geopotential = at(Altitude::GeometricMsl(10_000.0).geopotential_m());
    assert_eq!(geometric, geopotential);
    assert!(geometric.pressure_pa > at(10_000.0).pressure_pa);
}

#[test]
fn altitudes_outside_the_standard_are_refused() {
    for h in [-5_000.1, 80_000.1, f64::NAN] {
        assert!(matches!(
            Isa.at(Altitude::Geopotential(h)),
            Err(AtmosphereError::OutsideDomain { .. })
        ));
    }
}

#[test]
fn the_model_seam_answers_like_the_direct_call() {
    let point = AtmospherePoint {
        latitude_deg: 47.4,
        longitude_deg: 8.5,
        altitude: Altitude::GeometricMsl(488.0),
        at: DecimalYear(2026.8),
    };
    let model: &dyn AtmosphereModel = &Isa;
    assert_eq!(model.state(point), Isa.at(point.altitude));
    assert!((at(0.0).temperature_c() - 15.0).abs() < 1e-12);
}
