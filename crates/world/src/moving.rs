//! Tier-1 value-typed moving operational world (`Airmass` + `Surveillance` + `Notices`).

use world_airmass::{AirmassConfig, AirmassStore};
use world_core::DecimalYear;
use world_notices::{NoticesConfig, NoticesStore};
use world_surveillance::{SurveillanceConfig, SurveillanceStore};

/// Value-typed bundle of all perishable, moving operational world domains.
///
/// Because [`MovingWorld`] owns its stores by value with no process-global
/// state, callers instantiate distinct worlds for simulator physical truth and
/// per-vehicle avionics/datalink belief.
#[derive(Clone, Debug, Default)]
pub struct MovingWorld {
    /// Multi-layer weather soundings, surface observations, clouds and hazards.
    pub airmass: AirmassStore,
    /// Multi-source airborne and surface traffic tracks.
    pub surveillance: SurveillanceStore,
    /// Active TFRs, aerodrome/runway closures and SUA hot windows.
    pub notices: NoticesStore,
}

impl MovingWorld {
    /// Creates an empty [`MovingWorld`] with explicit store capacities and policies.
    pub fn with_configs(
        airmass: AirmassConfig,
        surveillance: SurveillanceConfig,
        notices: NoticesConfig,
    ) -> Self {
        Self {
            airmass: AirmassStore::with_config(airmass),
            surveillance: SurveillanceStore::with_config(surveillance),
            notices: NoticesStore::with_config(notices),
        }
    }

    /// Advances monotonic surveillance track freshness (`now_us`) and prunes
    /// expired weather and operational notices (`now_epoch`).
    pub fn advance(&mut self, now_us: u64, now_epoch: DecimalYear) {
        let _ = self.surveillance.advance_time(now_us);
        let _ = self.airmass.prune_expired(now_epoch);
        let _ = self.notices.prune_expired(now_epoch);
    }
}
