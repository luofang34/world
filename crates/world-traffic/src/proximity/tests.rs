extern crate std;

use std::vec::Vec;

use surveillance_core::{AddressNamespace, TrackKey};
use world_core::{GeodeticPosition, Ned};

use crate::fix::tests::{SECOND, engine, report};
use crate::{Ownship, nearest, traffic_by_range, within_radius};

const NEAR: TrackKey = TrackKey::new(AddressNamespace::Icao, 1);
const MIDDLE: TrackKey = TrackKey::new(AddressNamespace::Icao, 2);
const FAR: TrackKey = TrackKey::new(AddressNamespace::Icao, 3);
const SILENT: TrackKey = TrackKey::new(AddressNamespace::Icao, 4);

fn ownship() -> Ownship {
    Ownship {
        position: GeodeticPosition::wgs84(40.0, -75.0, 1_000.0),
        velocity_ned_m_s: Ned {
            north_m: 0.0,
            east_m: 0.0,
            down_m: 0.0,
        },
        pressure_altitude_ft: None,
    }
}

#[test]
fn tracks_rank_by_range_and_a_silent_one_is_left_out() {
    let mut engine = engine();
    // The silent track is the closest of all, but was last heard at 1 s.
    let reports = [
        report(SECOND, SILENT, (40.001, -75.0), (0.0, 0.0)),
        report(20 * SECOND, FAR, (40.30, -75.0), (90.0, 90.0)),
        report(20 * SECOND, NEAR, (40.02, -75.0), (90.0, 90.0)),
        report(20 * SECOND, MIDDLE, (40.10, -75.0), (90.0, 90.0)),
    ];
    for observation in reports {
        engine.ingest(observation, |_| {}).unwrap();
    }
    let now = 21 * SECOND;

    let ranked = traffic_by_range(&ownship(), engine.snapshots(), now);
    let keys: Vec<_> = ranked.iter().map(|relative| relative.key).collect();
    assert_eq!(keys, [NEAR, MIDDLE, FAR]);

    let two = nearest(&ownship(), engine.snapshots(), now, 2);
    assert_eq!(two.len(), 2);
    assert_eq!(two[1].key, MIDDLE);

    let close = within_radius(&ownship(), engine.snapshots(), now, 12_000.0);
    let keys: Vec<_> = close.iter().map(|relative| relative.key).collect();
    assert_eq!(keys, [NEAR, MIDDLE]);
}
