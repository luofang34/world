//! Weather queries over Airmass snapshots.
//!
//! Airmass owns weather state: ingress, source selection, revisions,
//! validity, expiry and the immutable [`WeatherSnapshot`] it publishes. This
//! crate holds no weather. It answers the questions a cockpit and a planner
//! ask of one snapshot: what a station reports ([`station_reports`],
//! [`reports_near`]) and which advisories apply at a point
//! ([`advisories_at`]) or along a route through space and time
//! ([`advisories_along_route`]).
//!
//! A snapshot fixes what was known; the query states the time asked about.
//! Every answer borrows the snapshot's own values, so each field keeps the
//! time evidence and [`ProductProvenance`] Airmass recorded for it. Where a
//! question cannot be settled from what Airmass publishes — an altitude band
//! on a reference the caller gave no altitude for, a station whose position
//! Airmass does not carry — the answer says so instead of guessing.
//!
//! `no_std` + `alloc`, depending on `airmass-core`, `world-core` and `libm`.
//!
//! [`WeatherSnapshot`]: airmass_core::WeatherSnapshot
//! [`ProductProvenance`]: airmass_core::ProductProvenance

#![no_std]

extern crate alloc;

mod advisory;
mod route;
mod station;

pub use advisory::{
    AdvisoryHit, AdvisoryQuery, HorizontalRelation, PublishedAdvisory, QueryAltitude,
    VerticalRelation, advisories, advisories_at,
};
pub use route::{RouteAdvisorySample, advisories_along_route};
pub use station::{
    LocatedReport, StationLocator, StationReport, reports_near, station_reports, text_reports,
};
