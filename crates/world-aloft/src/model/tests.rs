use alloc::vec;
use world_atmosphere::{Altitude, AtmosphereModel, AtmospherePoint, Isa};
use world_core::{DecimalYear, GeodeticPosition, Ned, ValidityWindow};

use crate::{AloftLevel, AloftModel, CloudGenus, CloudLayer, SoundingProfile, WindModel};

pub(crate) fn test_window() -> ValidityWindow {
    ValidityWindow {
        from: DecimalYear(2026.0),
        until: DecimalYear(2027.0),
    }
}

pub(crate) fn two_level_sounding(id: u64, valid: ValidityWindow) -> SoundingProfile {
    SoundingProfile::new(
        id,
        Some(GeodeticPosition::wgs84(39.18, -75.53, 0.0)),
        50_000.0,
        valid,
        vec![
            AloftLevel {
                altitude: Altitude::Geopotential(0.0),
                temperature_k: 298.15,
                pressure_pa: Some(101_000.0),
                wind_ned_m_s: Ned {
                    north_m: 4.0,
                    east_m: 2.0,
                    down_m: 0.0,
                },
                gust_m_s: 3.0,
                turbulence_edr: 0.05,
            },
            AloftLevel {
                altitude: Altitude::Geopotential(2_000.0),
                temperature_k: 282.15,
                pressure_pa: Some(79_500.0),
                wind_ned_m_s: Ned {
                    north_m: 12.0,
                    east_m: 18.0,
                    down_m: -0.4,
                },
                gust_m_s: 1.0,
                turbulence_edr: 0.15,
            },
        ],
    )
    .unwrap()
}

#[test]
fn empty_model_matches_isa_still_air() {
    let model = AloftModel::default();
    let pt = AtmospherePoint {
        latitude_deg: 39.18,
        longitude_deg: -75.53,
        altitude: Altitude::GeometricMsl(1_500.0),
        at: DecimalYear(2026.5),
    };
    let sample = model.sample(pt).unwrap();
    let isa = Isa.state(pt).unwrap();
    assert_eq!(sample.state, isa);
    assert_eq!(sample.wind_ned_m_s.north_m, 0.0);
    assert_eq!(sample.wind_ned_m_s.east_m, 0.0);
    assert!(sample.speed_of_sound_m_s > 330.0 && sample.speed_of_sound_m_s < 345.0);
}

#[test]
fn sounding_interpolates_multi_layer_winds_and_temperatures() {
    let mut model = AloftModel::default();
    model
        .upsert_sounding(two_level_sounding(1, test_window()))
        .unwrap();

    let mid = AtmospherePoint {
        latitude_deg: 39.20,
        longitude_deg: -75.50,
        altitude: Altitude::Geopotential(1_000.0),
        at: DecimalYear(2026.4),
    };
    let sample = model.sample(mid).unwrap();
    assert!((sample.state.temperature_k - 290.15).abs() < 1e-6);
    assert!((sample.wind_ned_m_s.north_m - 8.0).abs() < 1e-6);
    assert!((sample.wind_ned_m_s.east_m - 10.0).abs() < 1e-6);
    assert!((sample.gust_m_s - 2.0).abs() < 1e-6);

    // Outside radius falls back to ISA still air.
    let far = AtmospherePoint {
        latitude_deg: 45.0,
        ..mid
    };
    let far_sample = model.sample(far).unwrap();
    assert_eq!(far_sample.wind_ned_m_s.north_m, 0.0);
    assert_eq!(far_sample.state, Isa.state(far).unwrap());
}

#[test]
fn a_sounding_outside_its_validity_does_not_answer() {
    let mut model = AloftModel::default();
    model
        .upsert_sounding(two_level_sounding(1, test_window()))
        .unwrap();
    let after = AtmospherePoint {
        latitude_deg: 39.20,
        longitude_deg: -75.50,
        altitude: Altitude::Geopotential(1_000.0),
        at: DecimalYear(2027.5),
    };
    assert_eq!(model.sample(after).unwrap().wind_ned_m_s.north_m, 0.0);
}

#[test]
fn cloud_layers_answer_inside_their_region_and_expire() {
    let mut model = AloftModel::default();
    model
        .upsert_cloud_layer(CloudLayer {
            id: 7,
            center: Some(GeodeticPosition::wgs84(39.13, -75.47, 0.0)),
            radius_m: 25_000.0,
            coverage: 0.85,
            base_msl_m: 600.0,
            top_msl_m: 2_100.0,
            genus: CloudGenus::Stratiform,
            valid: test_window(),
        })
        .unwrap();
    let near = GeodeticPosition::wgs84(39.15, -75.48, 500.0);
    let cloud = model.cloud_at(near, DecimalYear(2026.5)).unwrap();
    assert_eq!(cloud.genus, CloudGenus::Stratiform);
    let far = GeodeticPosition::wgs84(41.0, -75.48, 500.0);
    assert!(model.cloud_at(far, DecimalYear(2026.5)).is_none());

    assert_eq!(model.prune_expired(DecimalYear(2028.0)), 1);
    assert!(model.cloud_at(near, DecimalYear(2026.5)).is_none());
}

#[test]
fn removing_a_sounding_restores_the_fallback() {
    let mut model = AloftModel::default();
    model
        .upsert_sounding(two_level_sounding(9, test_window()))
        .unwrap();
    assert!(model.sounding(9).is_some());
    assert!(model.remove_sounding(9));
    assert!(!model.remove_sounding(9));
    assert!(model.sounding(9).is_none());
}
