//! Multi-source airborne and surface surveillance tracks.
//!
//! [`SurveillanceStore`] is a value-typed, single-writer track store driven by
//! caller-supplied monotonic microseconds (`u64`), with no global state,
//! sockets, threads or system clocks. It fuses cooperative and non-cooperative
//! observations across ADS-B (1090ES / UAT 978), ADS-R, TIS-B, Mode-S/MLAT,
//! FLARM, ASTM F3411 Remote ID, C2 networks, radar and simulated traffic using
//! source-authority ranking and freshness windows, projects bounded dead
//! reckoning on the WGS-84 [`LocalTangentPlane`], and computes ownship-relative
//! geometry and closest-point-of-approach (CPA) kinematics without owning
//! cockpit alerting policy.
//!
//! `no_std` + `alloc`, depending only on `world-core` and `libm`.
//!
//! [`LocalTangentPlane`]: world_core::LocalTangentPlane

#![no_std]

extern crate alloc;

mod geometry;
mod identity;
mod observation;
mod store;

#[cfg(test)]
mod tests;

pub use geometry::{ProjectionBounds, RelativeTarget, project_geodetic, relative_target};
pub use identity::{AddressNamespace, Callsign, EmitterCategory, TrackId, TrackKey};
pub use observation::{
    DeliveryPath, IntegrityQuality, ObservationSource, TrackObservation, TrackPhase, TrackSnapshot,
};
pub use store::{
    FreshnessPolicy, IngestOutcome, SurveillanceConfig, SurveillanceError, SurveillanceStore,
};
