use world_core::{ClockCorrespondence, WorldInstant};

use crate::clock::{clock_correspondence, evaluation_time, time_correspondence};

#[test]
fn an_instant_reads_the_same_on_both_libraries_clocks() {
    let instant = WorldInstant::new(5_000_000, 1_767_225_600_000_999);
    let evaluation = evaluation_time(instant);
    assert_eq!(evaluation.monotonic.as_micros(), 5_000_000);
    // Rounded toward the past: 999 us is not yet the next millisecond.
    assert_eq!(evaluation.utc.as_unix_millis(), 1_767_225_600_000);

    let before_epoch = evaluation_time(WorldInstant::new(0, -1));
    assert_eq!(before_epoch.utc.as_unix_millis(), -1);
}

#[test]
fn a_correspondence_round_trips_through_the_surveillance_schema_type() {
    let clocks = ClockCorrespondence::new(5_000_000, 1_767_225_600_000_000);
    let stream = time_correspondence(clocks);
    // Surveillance's own conversion agrees with the workspace's.
    assert_eq!(
        stream.utc_unix_micros_at(65_000_000).unwrap(),
        clocks.at_monotonic(65_000_000).unwrap().utc_unix_micros
    );
    assert_eq!(clock_correspondence(stream), clocks);
}
