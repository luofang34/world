use alloc::vec;
use world_atmosphere::{Altitude, AtmosphereModel, AtmospherePoint, Isa};
use world_core::{DecimalYear, GeodeticPosition, Ned, ValidityWindow};

use crate::{
    AirmassStore, AloftLevel, CloudGenus, CloudLayer, FlightCategory, HazardGeometry, HazardKind,
    IcingSeverity, SoundingProfile, StationId, SurfaceObservation, TurbulenceSeverity,
    WeatherHazard, WindModel,
};

fn test_window() -> ValidityWindow {
    ValidityWindow {
        from: DecimalYear(2026.0),
        until: DecimalYear(2027.0),
    }
}

#[test]
fn empty_store_matches_isa_still_air() {
    let store = AirmassStore::default();
    let pt = AtmospherePoint {
        latitude_deg: 39.18,
        longitude_deg: -75.53,
        altitude: Altitude::GeometricMsl(1_500.0),
        at: DecimalYear(2026.5),
    };
    let sample = store.sample(pt).unwrap();
    let isa = Isa.state(pt).unwrap();
    assert_eq!(sample.state, isa);
    assert_eq!(sample.wind_ned_m_s.north_m, 0.0);
    assert_eq!(sample.wind_ned_m_s.east_m, 0.0);
    assert!(sample.speed_of_sound_m_s > 330.0 && sample.speed_of_sound_m_s < 345.0);
}

#[test]
fn sounding_interpolates_multi_layer_winds_and_temperatures() {
    let mut store = AirmassStore::default();
    let anchor = GeodeticPosition::wgs84(39.18, -75.53, 0.0);
    let profile = SoundingProfile::new(
        1,
        Some(anchor),
        50_000.0,
        test_window(),
        None,
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
    .unwrap();
    store.upsert_sounding(profile).unwrap();

    let mid = AtmospherePoint {
        latitude_deg: 39.20,
        longitude_deg: -75.50,
        altitude: Altitude::Geopotential(1_000.0),
        at: DecimalYear(2026.4),
    };
    let sample = store.sample(mid).unwrap();
    assert!((sample.state.temperature_k - 290.15).abs() < 1e-6);
    assert!((sample.wind_ned_m_s.north_m - 8.0).abs() < 1e-6);
    assert!((sample.wind_ned_m_s.east_m - 10.0).abs() < 1e-6);
    assert!((sample.gust_m_s - 2.0).abs() < 1e-6);

    // Outside radius falls back to ISA still air.
    let far = AtmospherePoint {
        latitude_deg: 45.0,
        longitude_deg: -75.50,
        altitude: Altitude::Geopotential(1_000.0),
        at: DecimalYear(2026.4),
    };
    let far_sample = store.sample(far).unwrap();
    assert_eq!(far_sample.wind_ned_m_s.north_m, 0.0);
    assert_eq!(far_sample.state, Isa.state(far).unwrap());
}

#[test]
fn observations_and_clouds_work() {
    let mut store = AirmassStore::default();
    let kdov = StationId::parse("kdov").unwrap();
    assert_eq!(kdov.as_str(), "KDOV");
    let cat = FlightCategory::from_ceiling_and_visibility(Some(240.0), 4_000.0);
    assert_eq!(cat, FlightCategory::Ifr);

    let obs = SurfaceObservation {
        station: kdov,
        position: Some(GeodeticPosition::wgs84(39.13, -75.47, 9.0)),
        observed_at: DecimalYear(2026.3),
        valid: test_window(),
        wind_ned_m_s: Ned {
            north_m: -5.0,
            east_m: 3.0,
            down_m: 0.0,
        },
        gust_m_s: 2.0,
        visibility_m: 4_000.0,
        ceiling_agl_m: Some(240.0),
        temperature_k: 289.15,
        dewpoint_k: 287.15,
        altimeter_pa: 101_450.0,
        flight_category: cat,
    };
    assert!(store.upsert_observation(obs).unwrap());
    let stale_obs = SurfaceObservation {
        observed_at: DecimalYear(2026.2),
        ..obs
    };
    assert!(!store.upsert_observation(stale_obs).unwrap());
    assert!(store.observation(kdov, DecimalYear(2026.5)).is_some());

    store
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
    let cloud = store
        .cloud_at(
            GeodeticPosition::wgs84(39.15, -75.48, 500.0),
            DecimalYear(2026.5),
        )
        .unwrap();
    assert_eq!(cloud.genus, CloudGenus::Stratiform);
}

#[test]
fn polygon_hazards_and_expiry_pruning_work() {
    let mut store = AirmassStore::default();
    store
        .upsert_hazard(WeatherHazard {
            id: 42,
            kind: HazardKind::ConvectiveSigmet,
            geometry: HazardGeometry::Polygon {
                vertices: vec![
                    GeodeticPosition::wgs84(39.0, -75.7, 0.0),
                    GeodeticPosition::wgs84(39.0, -75.2, 0.0),
                    GeodeticPosition::wgs84(39.4, -75.2, 0.0),
                    GeodeticPosition::wgs84(39.4, -75.7, 0.0),
                ],
            },
            floor_msl_m: 500.0,
            ceiling_msl_m: 11_000.0,
            valid: test_window(),
            icing: IcingSeverity::Moderate,
            turbulence: TurbulenceSeverity::Severe,
            provenance: None,
        })
        .unwrap();
    let hits = store.active_hazards_at(
        GeodeticPosition::wgs84(39.2, -75.45, 2_000.0),
        2_000.0,
        DecimalYear(2026.5),
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].turbulence, TurbulenceSeverity::Severe);

    let pruned = store.prune_expired(DecimalYear(2028.0));
    assert_eq!(pruned, 1);
}
