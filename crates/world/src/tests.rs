#[cfg(all(feature = "atmosphere", feature = "geomag"))]
#[test]
fn standard_world_evaluates_atmosphere_geomag_and_gravity() {
    use crate::{DecimalYear, GeodeticPosition, StandardWorld};

    let world = StandardWorld;
    let pos = GeodeticPosition::wgs84(39.18, -75.53, 900.0);
    let at = DecimalYear(2026.5);

    let air = world.atmosphere_at(pos, at).unwrap();
    assert!(air.density_kg_m3 > 1.0 && air.density_kg_m3 < 1.25);

    let mag = world.magnetic_field_at(pos, at).unwrap();
    assert!(mag.declination_rad().is_finite());

    let g = world.gravity_m_s2(pos).unwrap();
    assert!((g - 9.8).abs() < 0.05);
}

#[cfg(feature = "moving")]
#[test]
fn truth_and_belief_moving_worlds_are_independent_values() {
    use crate::airmass::{AloftLevel, SoundingProfile, WindModel};
    use crate::atmosphere::{Altitude, AtmospherePoint};
    use crate::{DecimalYear, MovingWorld, Ned, ValidityWindow};

    let mut truth = MovingWorld::default();
    let belief = MovingWorld::default();

    let profile = SoundingProfile::new(
        1,
        None,
        0.0,
        ValidityWindow {
            from: DecimalYear(2026.0),
            until: DecimalYear(2027.0),
        },
        None,
        [AloftLevel {
            altitude: Altitude::Geopotential(1_000.0),
            temperature_k: 285.0,
            pressure_pa: None,
            wind_ned_m_s: Ned {
                north_m: 14.0,
                east_m: -6.0,
                down_m: 0.0,
            },
            gust_m_s: 0.0,
            turbulence_edr: 0.0,
        }]
        .into(),
    )
    .unwrap();
    truth.airmass.upsert_sounding(profile).unwrap();

    let query = AtmospherePoint {
        latitude_deg: 39.18,
        longitude_deg: -75.53,
        altitude: Altitude::Geopotential(1_000.0),
        at: DecimalYear(2026.5),
    };
    assert_eq!(
        truth.airmass.sample(query).unwrap().wind_ned_m_s.north_m,
        14.0
    );
    assert_eq!(
        belief.airmass.sample(query).unwrap().wind_ned_m_s.north_m,
        0.0
    );
}
