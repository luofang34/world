//! The model against physics it was not transcribed from: hydrostatic and
//! diffusive equilibrium, and the response to solar and geomagnetic
//! activity.

use world_core::{GeodeticPosition, normal_gravity};
use world_thermosphere::{GeomagneticActivity, MsisInput, Nrlmsise00, ThermosphereState};

/// Boltzmann constant, J/K (2019 SI).
const BOLTZMANN: f64 = 1.380_649e-23;
/// Atomic mass constant, kg (CODATA 2018).
const AMU: f64 = 1.660_539_066_60e-27;
/// Mean Earth radius, m, for the fall of gravity with height.
const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// One species' number density, read from a state.
type Density = fn(&ThermosphereState) -> f64;

fn input(altitude_km: f64) -> MsisInput {
    MsisInput {
        position: GeodeticPosition::wgs84(35.0, 10.0, altitude_km * 1000.0),
        day_of_year: 100,
        ut_seconds: 43_200.0,
        local_solar_time_h: MsisInput::apparent_local_solar_time_h(43_200.0, 10.0),
        f107_previous_day: 140.0,
        f107_81_day_mean: 140.0,
        geomagnetic: GeomagneticActivity::DailyAp(8.0),
    }
}

#[cfg(test)]
fn state(input: &MsisInput) -> ThermosphereState {
    Nrlmsise00::STANDARD.evaluate(input).expect("a state")
}

/// Gravity at `altitude_km` above the test position, m/s².
#[cfg(test)]
fn gravity(altitude_km: f64) -> f64 {
    let surface = normal_gravity(GeodeticPosition::wgs84(35.0, 10.0, 0.0)).expect("a position");
    let ratio = EARTH_RADIUS_M / (EARTH_RADIUS_M + altitude_km * 1000.0);
    surface * ratio * ratio
}

/// The scale height, m, a density falls off with between two altitudes.
fn measured_scale_height(
    low_km: f64,
    high_km: f64,
    density: impl Fn(&ThermosphereState) -> f64,
) -> f64 {
    let low = density(&state(&input(low_km)));
    let high = density(&state(&input(high_km)));
    (high_km - low_km) * 1000.0 / (low / high).ln()
}

#[test]
fn each_species_falls_off_with_its_own_scale_height_in_the_exosphere() {
    // Above 500 km the temperature is the exospheric one and every species
    // is in diffusive equilibrium: n falls as exp(-z / H), H = kT / (m g).
    let (low_km, high_km) = (600.0, 610.0);
    let middle = state(&input(605.0));
    assert!((middle.temperature_k - middle.exospheric_temperature_k).abs() < 0.5);
    let species: [(f64, Density); 5] = [
        (1.0, |s| s.number_density.atomic_hydrogen_m3),
        (4.0, |s| s.number_density.helium_m3),
        (14.0, |s| s.number_density.atomic_nitrogen_m3),
        (16.0, |s| s.number_density.atomic_oxygen_m3),
        (28.0, |s| s.number_density.molecular_nitrogen_m3),
    ];
    for (mass, density) in species {
        let expected = BOLTZMANN * middle.temperature_k / (mass * AMU * gravity(605.0));
        let measured = measured_scale_height(low_km, high_km, density);
        assert!(
            (measured / expected - 1.0).abs() < 0.01,
            "mass {mass}: measured {measured} m, kT/mg {expected} m"
        );
    }
}

#[test]
fn the_mixed_atmosphere_is_in_hydrostatic_balance() {
    // Below the turbopause dp/dz = -rho g with p = n k T.
    for altitude_km in [5.0, 20.0, 40.0, 60.0] {
        let pressure = |z: f64| {
            let s = state(&input(z));
            let n = s.number_density;
            let total = n.helium_m3
                + n.atomic_oxygen_m3
                + n.molecular_nitrogen_m3
                + n.molecular_oxygen_m3
                + n.argon_m3
                + n.atomic_hydrogen_m3
                + n.atomic_nitrogen_m3;
            total * BOLTZMANN * s.temperature_k
        };
        let step_km = 0.05;
        let gradient = (pressure(altitude_km + step_km) - pressure(altitude_km - step_km))
            / (2.0 * step_km * 1000.0);
        let weight = state(&input(altitude_km)).mass_density_kg_m3 * gravity(altitude_km);
        assert!(
            (-gradient / weight - 1.0).abs() < 0.01,
            "{altitude_km} km: -dp/dz {} against rho g {weight}",
            -gradient
        );
    }
}

#[test]
fn the_surface_is_close_to_the_standard_atmosphere() {
    let surface = state(&input(0.0));
    assert!((surface.temperature_k - 288.15).abs() < 15.0);
    assert!((surface.mass_density_kg_m3 - 1.225).abs() < 0.1);
    let n = surface.number_density;
    assert_eq!(n.atomic_oxygen_m3, 0.0);
    assert_eq!(n.atomic_hydrogen_m3, 0.0);
    assert_eq!(n.atomic_nitrogen_m3, 0.0);
    let oxygen_fraction = n.molecular_oxygen_m3
        / (n.molecular_nitrogen_m3 + n.molecular_oxygen_m3 + n.argon_m3 + n.helium_m3);
    assert!((oxygen_fraction - 0.2095).abs() < 0.001);
}

#[test]
fn the_density_falls_with_altitude_all_the_way_up() {
    let mut previous = f64::INFINITY;
    for step in 0..=200 {
        let density = state(&input(f64::from(step) * 5.0)).mass_density_kg_m3;
        assert!(density < previous, "at {} km", step * 5);
        previous = density;
    }
}

#[test]
fn a_more_active_sun_heats_and_thickens_the_thermosphere() {
    for altitude_km in [250.0, 400.0, 800.0] {
        let mut previous: Option<ThermosphereState> = None;
        for flux in [70.0, 100.0, 130.0, 160.0, 190.0, 220.0, 250.0] {
            let mut input = input(altitude_km);
            input.f107_previous_day = flux;
            input.f107_81_day_mean = flux;
            let current = state(&input);
            if let Some(previous) = previous {
                assert!(current.exospheric_temperature_k > previous.exospheric_temperature_k);
                assert!(
                    current.mass_density_kg_m3 > previous.mass_density_kg_m3,
                    "{altitude_km} km at F10.7 {flux}"
                );
            }
            previous = Some(current);
        }
    }
}

#[test]
fn geomagnetic_activity_heats_and_thickens_the_thermosphere() {
    let mut previous: Option<ThermosphereState> = None;
    for ap in [0.0, 4.0, 15.0, 48.0, 100.0, 200.0, 400.0] {
        let mut input = input(400.0);
        input.geomagnetic = GeomagneticActivity::DailyAp(ap);
        let current = state(&input);
        if let Some(previous) = previous {
            assert!(current.exospheric_temperature_k > previous.exospheric_temperature_k);
            assert!(current.mass_density_kg_m3 > previous.mass_density_kg_m3);
        }
        previous = Some(current);
    }
}

#[test]
fn the_profile_has_no_large_step_where_its_formulations_join() {
    // 32.5 km and 72.5 km join spline segments, 120 km joins the Bates
    // profile, and the other altitudes are where a species' mixing ends.
    // The model is not exactly continuous: just above 72.5 km the
    // temperature-gradient variations switch on, and each mixing
    // correction ends at a finite size.
    for join_km in [
        32.5, 72.5, 120.0, 160.0, 200.0, 240.0, 250.0, 300.0, 320.0, 450.0,
    ] {
        let below = state(&input(join_km - 1e-4));
        let at = state(&input(join_km));
        let above = state(&input(join_km + 1e-4));
        for (a, b) in [(&below, &at), (&at, &above)] {
            assert!(
                (a.mass_density_kg_m3 / b.mass_density_kg_m3 - 1.0).abs() < 5e-3,
                "density across {join_km} km"
            );
            assert!(
                (a.temperature_k - b.temperature_k).abs() < 1e-2,
                "temperature across {join_km} km"
            );
        }
    }
}

#[test]
fn anomalous_oxygen_matters_to_drag_only_high_up() {
    let low = state(&input(300.0));
    assert!(low.drag_mass_density_kg_m3 / low.mass_density_kg_m3 - 1.0 < 1e-6);
    let high = state(&input(1_500.0));
    assert!(high.drag_mass_density_kg_m3 / high.mass_density_kg_m3 - 1.0 > 0.01);
}
