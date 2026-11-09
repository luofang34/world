use world_core::{GeodeticPosition, LocalTangentPlane, Ned};

use crate::{
    AddressNamespace, Callsign, DeliveryPath, EmitterCategory, IngestOutcome, IntegrityQuality,
    ObservationSource, SurveillanceStore, TrackKey, TrackObservation, TrackPhase,
};

fn sample_obs(
    key: TrackKey,
    source: ObservationSource,
    observed_at_us: u64,
    pos: GeodeticPosition,
    vel: Ned,
) -> TrackObservation {
    TrackObservation {
        key,
        source,
        delivery: DeliveryPath::LocalReceiver,
        observed_at_us,
        position: pos,
        velocity_ned_m_s: vel,
        heading_rad: Some(0.0),
        callsign: Callsign::parse("N172SP"),
        category: EmitterCategory::Light,
        on_ground: false,
        quality: IntegrityQuality {
            nic: 8,
            nac_p: 9,
            sil: 3,
            horizontal_1sigma_m: None,
        },
    }
}

#[test]
fn authority_ranking_blocks_lower_rank_overwrite_when_fresh() {
    let mut store = SurveillanceStore::default();
    let key = TrackKey {
        namespace: AddressNamespace::Icao24,
        address: 0xA12345,
    };
    let pos = GeodeticPosition::wgs84(39.18, -75.53, 900.0);
    let vel = Ned {
        north_m: 50.0,
        east_m: 0.0,
        down_m: 0.0,
    };

    let created = store
        .ingest(
            sample_obs(key, ObservationSource::Adsb1090, 1_000_000, pos, vel),
            1_000_000,
        )
        .unwrap();
    let IngestOutcome::Created(id) = created else {
        panic!("expected Created");
    };

    // Lower-ranked TIS-B 2 seconds later must not overwrite fresh ADS-B 1090ES.
    let tisb_pos = GeodeticPosition::wgs84(39.19, -75.53, 920.0);
    let ignored = store
        .ingest(
            sample_obs(key, ObservationSource::TisB, 3_000_000, tisb_pos, vel),
            3_000_000,
        )
        .unwrap();
    assert_eq!(ignored, IngestOutcome::IgnoredLowerAuthority(id));
    assert_eq!(store.track(id).unwrap().source, ObservationSource::Adsb1090);

    // Once ADS-B goes stale (> 15 s), TIS-B is accepted.
    let updated = store
        .ingest(
            sample_obs(key, ObservationSource::TisB, 20_000_000, tisb_pos, vel),
            20_000_000,
        )
        .unwrap();
    assert_eq!(updated, IngestOutcome::Updated(id));
    assert_eq!(store.track(id).unwrap().source, ObservationSource::TisB);
}

#[test]
fn flarm_remote_id_coasting_expiry_and_cpa_geometry() {
    let mut store = SurveillanceStore::default();
    let ownship_key = TrackKey {
        namespace: AddressNamespace::Icao24,
        address: 0xABCDEF,
    };
    store.set_ownship_keys(&[ownship_key]);

    let ownship_pos = GeodeticPosition::wgs84(39.18, -75.53, 500.0);
    let ownship_vel = Ned {
        north_m: 40.0,
        east_m: 0.0,
        down_m: 0.0,
    };
    let plane = LocalTangentPlane::new(ownship_pos).unwrap();

    // Ownship shadow track should be tagged and excluded from relative intruder list.
    store
        .ingest(
            sample_obs(
                ownship_key,
                ObservationSource::Adsb1090,
                1_000_000,
                ownship_pos,
                ownship_vel,
            ),
            1_000_000,
        )
        .unwrap();
    assert!(store.track_by_key(ownship_key).unwrap().ownship_shadow);

    // Head-on drone 1,000 m north flying south at 20 m/s (closing at 60 m/s -> CPA in ~16.67 s).
    let drone_key = TrackKey {
        namespace: AddressNamespace::RemoteId,
        address: 0x9901,
    };
    let drone_pos = plane.to_geodetic(Ned {
        north_m: 1_000.0,
        east_m: 0.0,
        down_m: 0.0,
    });
    let mut drone_obs = sample_obs(
        drone_key,
        ObservationSource::RemoteId,
        1_000_000,
        drone_pos,
        Ned {
            north_m: -20.0,
            east_m: 0.0,
            down_m: 0.0,
        },
    );
    drone_obs.category = EmitterCategory::Uav;
    store.ingest(drone_obs, 1_000_000).unwrap();

    let rel = store.relative_to_ownship(ownship_pos, ownship_vel, 1_000_000);
    assert_eq!(rel.len(), 1);
    assert!((rel[0].horizontal_range_m - 1_000.0).abs() < 0.5);
    assert!((rel[0].closure_rate_m_s - 60.0).abs() < 0.5);
    assert!((rel[0].time_to_cpa_s - (1_000.0 / 60.0)).abs() < 0.2);
    assert!(rel[0].cpa_horizontal_m < 1.0);

    // Advance past 15s -> Coasting; past 60s -> Expired.
    assert_eq!(store.advance_time(17_000_000), 0);
    assert_eq!(
        store.track_by_key(drone_key).unwrap().phase,
        TrackPhase::Coasting
    );
    assert_eq!(store.advance_time(62_000_000), 2);
    assert!(store.tracks().is_empty());
}
