//! Temporary Flight Restriction (TFR) notices.

use world_core::{DecimalYear, GeodeticPosition, Provenance, ValidityWindow};

use crate::{LateralShape, NoticeId, VerticalBand};

/// Regulatory / operational reason for a Temporary Flight Restriction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TfrKind {
    /// Presidential or VIP movement (e.g. 14 CFR 91.141).
    Vip,
    /// Disaster relief, wildfire suppression or toxic hazard (e.g. 14 CFR 91.137).
    HazardDisaster,
    /// Major sporting event or stadium gathering (e.g. 99.7).
    StadiumSporting,
    /// Rocket launch, reentry or space operations (e.g. 14 CFR 91.143).
    SpaceOperations,
    /// National defense / security restriction (e.g. 14 CFR 99.7).
    SecurityDefense,
    /// Airshow or aerial demonstration (e.g. 14 CFR 91.145).
    AerialDemonstration,
    /// Other temporary airspace restriction.
    Special,
}

/// Perishable Temporary Flight Restriction volume.
#[derive(Clone, Debug, PartialEq)]
pub struct TfrNotice {
    /// NOTAM / TFR identifier.
    pub id: NoticeId,
    /// Restriction category.
    pub kind: TfrKind,
    /// Horizontal footprint.
    pub lateral: LateralShape,
    /// Vertical altitude limits.
    pub vertical: VerticalBand,
    /// Effective interval.
    pub valid: ValidityWindow,
    /// Explicit cancellation flag.
    pub cancelled: bool,
    /// Optional dataset / feed provenance.
    pub provenance: Option<Provenance>,
}

impl TfrNotice {
    /// Whether this TFR is active (not cancelled and within `valid`) at `at`.
    pub fn is_active_at(&self, at: DecimalYear) -> bool {
        !self.cancelled && self.valid.contains(at)
    }

    /// Whether `position` at `altitude_msl_m` and `at` penetrates this active TFR.
    pub fn contains(
        &self,
        position: GeodeticPosition,
        altitude_msl_m: f64,
        at: DecimalYear,
    ) -> bool {
        self.is_active_at(at)
            && self.vertical.contains_msl_m(altitude_msl_m)
            && self.lateral.contains(position)
    }
}
