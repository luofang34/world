//! Surveillance observation origins, delivery paths, integrity and track snapshots.

use world_core::{GeodeticPosition, Ned, Uncertainty};

use crate::{Callsign, EmitterCategory, TrackId, TrackKey};

/// Surveillance sensor or broadcast technology producing an observation.
///
/// [`ObservationSource::authority_rank`] defines deterministic fusion precedence
/// among fresh candidates; [`DeliveryPath`] never grants fusion authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObservationSource {
    /// Direct 1090 MHz Extended Squitter ADS-B reception.
    Adsb1090,
    /// Direct 978 MHz Universal Access Transceiver ADS-B reception.
    Uat978,
    /// Mode-S reply or wide-area multilateration (MLAT).
    ModeSMlat,
    /// Automatic Dependent Surveillance – Rebroadcast (ADS-R).
    AdsR,
    /// FLARM peer-to-peer VFR/glider collision-avoidance radio.
    Flarm,
    /// ASTM F3411 / OpenDroneID broadcast or network Remote ID.
    RemoteId,
    /// Primary or secondary surveillance radar track.
    PrimaryRadar,
    /// Tactical / C2 network fused track.
    C2Network,
    /// Traffic Information Service – Broadcast (TIS-B).
    TisB,
    /// Direct truth or sensor-modelled simulator target.
    Simulated,
    /// Unclassified source.
    Unknown,
}

impl ObservationSource {
    /// Fusion authority rank among fresh observations (higher wins).
    pub const fn authority_rank(self) -> u8 {
        match self {
            Self::Simulated => 10,
            Self::Adsb1090 | Self::Uat978 => 9,
            Self::Flarm | Self::RemoteId => 8,
            Self::AdsR => 7,
            Self::ModeSMlat => 6,
            Self::PrimaryRadar => 5,
            Self::TisB => 4,
            Self::C2Network => 3,
            Self::Unknown => 0,
        }
    }
}

/// Transport path by which an observation reached the store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeliveryPath {
    /// Unspecified transport.
    #[default]
    Unspecified,
    /// Portable or direct local SDR/radio receiver.
    LocalReceiver,
    /// Certified installed avionics bus.
    InstalledAvionics,
    /// Ground/cloud network feed.
    NetworkFeed,
    /// Recorded telemetry replay.
    Replay,
    /// Closed-loop or interactive simulator injection.
    Simulator,
}

/// Reported navigation integrity and accuracy indicators.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntegrityQuality {
    /// Navigation Integrity Category (0..=11).
    pub nic: u8,
    /// Navigation Accuracy Category for Position (0..=11).
    pub nac_p: u8,
    /// Source Integrity Level (0..=3).
    pub sil: u8,
    /// Optional 1-sigma horizontal position uncertainty, meters.
    pub horizontal_1sigma_m: Option<Uncertainty>,
}

/// Lifecycle phase of a tracked target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackPhase {
    /// Fresh position update received within the active freshness window.
    Active,
    /// No fresh update since `coast_after_us`, but not yet expired.
    Coasting,
}

/// One incoming surveillance observation at monotonic timestamp `observed_at_us`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackObservation {
    /// External target key.
    pub key: TrackKey,
    /// Sensor / broadcast origin.
    pub source: ObservationSource,
    /// Transport path.
    pub delivery: DeliveryPath,
    /// Monotonic observation timestamp in microseconds.
    pub observed_at_us: u64,
    /// Reported geodetic position.
    pub position: GeodeticPosition,
    /// Reported local north-east-down velocity, m/s.
    pub velocity_ned_m_s: Ned,
    /// Optional true ground track or heading in radians `[0, 2π)`.
    pub heading_rad: Option<f64>,
    /// Optional callsign / flight ID.
    pub callsign: Option<Callsign>,
    /// Emitter category.
    pub category: EmitterCategory,
    /// Airborne vs on-ground flag.
    pub on_ground: bool,
    /// Reported integrity / accuracy indicators.
    pub quality: IntegrityQuality,
}

/// Fused state of one surveillance track held in [`crate::SurveillanceStore`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSnapshot {
    /// Stable store-assigned track identifier.
    pub id: TrackId,
    /// Primary external target key.
    pub key: TrackKey,
    /// Lifecycle phase (`Active` vs `Coasting`).
    pub phase: TrackPhase,
    /// Authoritative source of the current kinematic state.
    pub source: ObservationSource,
    /// Delivery path of the authoritative update.
    pub delivery: DeliveryPath,
    /// Monotonic timestamp of the latest accepted kinematic update, microseconds.
    pub observed_at_us: u64,
    /// Geodetic position at `observed_at_us`.
    pub position: GeodeticPosition,
    /// Local north-east-down velocity at `observed_at_us`, m/s.
    pub velocity_ned_m_s: Ned,
    /// Optional true heading / ground track in radians.
    pub heading_rad: Option<f64>,
    /// Latest known callsign, retained across kinematic-only frames.
    pub callsign: Option<Callsign>,
    /// Emitter category.
    pub category: EmitterCategory,
    /// Whether the target reported an on-ground state.
    pub on_ground: bool,
    /// Reported integrity / accuracy indicators.
    pub quality: IntegrityQuality,
    /// True when `key` matches a configured ownship identity (tagged, never silently dropped).
    pub ownship_shadow: bool,
}
