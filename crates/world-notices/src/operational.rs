//! Aerodrome surface closures and Special Use Airspace (SUA) activation notices.

use world_core::ValidityWindow;

use crate::{AirspaceKey, FacilityKey, NoticeId, RunwayKey, VerticalBand};

/// Aerodrome element affected by a surface operational notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceElement {
    /// Entire aerodrome or heliport.
    Aerodrome,
    /// Specific runway or runway end.
    Runway(RunwayKey),
    /// Taxiway network element.
    Taxiway,
    /// Ramp / apron parking area.
    Apron,
}

/// Operational status reported by a surface notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureStatus {
    /// Closed to all operations.
    Closed,
    /// Prior permission required or weight/aircraft-category restricted.
    Restricted,
    /// Standing water, snow, slush or degraded braking action.
    Contaminated,
}

/// Perishable NOTAM affecting an aerodrome or runway surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceClosureNotice {
    /// NOTAM identifier.
    pub id: NoticeId,
    /// Aerodrome / heliport identifier (`Flightbag` facility key).
    pub facility: FacilityKey,
    /// Specific surface element affected.
    pub element: SurfaceElement,
    /// Operational restriction status.
    pub status: ClosureStatus,
    /// Effective interval.
    pub valid: ValidityWindow,
    /// Explicit cancellation flag.
    pub cancelled: bool,
}

/// Kind of Special Use Airspace activated by schedule or NOTAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SuaKind {
    /// Military Operations Area.
    Moa,
    /// Restricted Area.
    Restricted,
    /// Warning Area.
    Warning,
    /// Alert Area.
    Alert,
    /// Prohibited Area.
    Prohibited,
    /// Controlled Firing Area.
    ControlledFiring,
}

/// Perishable hot-window activation of a charted Special Use Airspace volume.
///
/// Static lateral boundaries of the SUA live in `Flightbag` keyed by [`AirspaceKey`];
/// this notice carries the live activation schedule and active altitude band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuaActivationNotice {
    /// NOTAM or schedule identifier.
    pub id: NoticeId,
    /// Charted SUA identifier (`Flightbag` airspace key).
    pub airspace: AirspaceKey,
    /// SUA classification.
    pub kind: SuaKind,
    /// Active vertical band during this window.
    pub vertical: VerticalBand,
    /// Effective activation window.
    pub valid: ValidityWindow,
    /// Explicit cancellation flag.
    pub cancelled: bool,
}
