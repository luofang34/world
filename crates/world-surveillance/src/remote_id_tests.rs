use remoteid::{BasicId, IdType, Location, Message, Status, UaType, UasData};

use crate::{
    AddressNamespace, DeliveryPath, EmitterCategory, IngestOutcome, ObservationSource,
    SurveillanceStore, TrackObservation,
};

#[test]
fn remote_id_frames_become_a_uav_track() {
    let decoded = |msg: &Message| Message::decode(&msg.encode().unwrap()).unwrap();
    let mut uas = UasData::default();
    let id = BasicId::new(
        IdType::SerialNumber,
        UaType::HelicopterOrMultirotor,
        "1596F3507452",
    )
    .unwrap();
    assert!(uas.update(&decoded(&Message::BasicId(id))));
    let loc = Location {
        status: Status::Airborne,
        direction: Some(90.0),
        speed_horizontal: Some(12.0),
        speed_vertical: Some(2.0),
        latitude: 39.18,
        longitude: -75.53,
        altitude_geo: Some(120.0),
        horizontal_accuracy: remoteid::HorizontalAccuracy(10),
        ..Location::default()
    };
    assert!(uas.update(&decoded(&Message::Location(loc))));

    let obs =
        TrackObservation::from_remote_id(&uas, DeliveryPath::LocalReceiver, 5_000_000).unwrap();
    assert_eq!(obs.key.namespace, AddressNamespace::RemoteId);
    assert_eq!(obs.source, ObservationSource::RemoteId);
    assert_eq!(obs.category, EmitterCategory::Uav);
    assert!(!obs.on_ground);
    assert_eq!(obs.quality.nac_p, 10);
    assert!((obs.position.latitude_deg - 39.18).abs() < 1e-6);
    assert!((obs.position.ellipsoidal_height_m - 120.0).abs() < 0.5);
    assert!(obs.velocity_ned_m_s.north_m.abs() < 1e-3);
    assert!((obs.velocity_ned_m_s.east_m - 12.0).abs() < 1e-3);
    assert!((obs.velocity_ned_m_s.down_m + 2.0).abs() < 1e-3);

    let mut store = SurveillanceStore::default();
    let IngestOutcome::Created(track) = store.ingest(obs, 5_000_000).unwrap() else {
        panic!("expected Created");
    };
    assert_eq!(store.track_by_key(obs.key).map(|t| t.id), Some(track));
}

#[test]
fn remote_id_without_position_or_id_is_not_an_observation() {
    let mut uas = UasData::default();
    assert!(TrackObservation::from_remote_id(&uas, DeliveryPath::LocalReceiver, 0).is_none());
    let id = BasicId::new(IdType::SerialNumber, UaType::Aeroplane, "ABC123").unwrap();
    uas.update(&Message::BasicId(id));
    uas.update(&Message::Location(Location {
        altitude_geo: Some(50.0),
        ..Location::default()
    }));
    assert!(TrackObservation::from_remote_id(&uas, DeliveryPath::LocalReceiver, 0).is_none());
}
