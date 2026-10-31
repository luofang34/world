//! Perishable operational airspace and aerodrome state: Temporary Flight
//! Restrictions (TFRs), live runway/aerodrome closure NOTAMs and Special Use
//! Airspace (SUA) hot/cold activation schedules.
//!
//! Standing, cycle-dated aeronautical baseline data (charts, runway geometry,
//! permanent Special Flight Rules Areas) lives in `Flightbag`; [`NoticesStore`]
//! holds the moving, time-bounded operational restrictions and binds to static
//! aerodromes and airspaces by lightweight value keys ([`FacilityKey`],
//! [`RunwayKey`], [`AirspaceKey`]) without depending on `Flightbag`.
//!
//! `no_std` + `alloc`, depending only on `world-core`, `world-atmosphere` and
//! `libm`.

#![no_std]

extern crate alloc;

mod boundary;
mod keys;
mod operational;
mod store;
mod tfr;

#[cfg(test)]
mod tests;

pub use boundary::{LateralShape, NoticeAltitude, VerticalBand};
pub use keys::{AirspaceKey, FacilityKey, NoticeError, NoticeId, RunwayKey};
pub use operational::{
    ClosureStatus, SuaActivationNotice, SuaKind, SurfaceClosureNotice, SurfaceElement,
};
pub use store::{NoticesConfig, NoticesStore};
pub use tfr::{TfrKind, TfrNotice};
