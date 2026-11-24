//! What a moving world is built from, and what advancing it reports.

use airmass_core::{AdvanceOutcome, StoreConfig};
use surveillance_core::EngineConfig;
use world_aloft::AloftConfig;
use world_notices::NoticesConfig;

/// Configuration and producer identities for one [`crate::MovingWorld`].
///
/// Each library asks its host for a new producer identity per constructed
/// instance and has no clock or random source to make one. The identities
/// are therefore the caller's to supply: a truth world and a belief world
/// must not share them.
#[derive(Clone, Debug, PartialEq)]
pub struct MovingWorldConfig {
    /// Surveillance engine tuning.
    pub surveillance: EngineConfig,
    /// Identity of this world's Surveillance engine instance.
    pub surveillance_producer: surveillance_core::ProducerInstanceId,
    /// Airmass store capacity and source preference.
    pub weather: StoreConfig,
    /// Identity of this world's Airmass store instance.
    pub weather_producer: airmass_core::ProducerInstanceId,
    /// Notices store capacity.
    pub notices: NoticesConfig,
    /// Aloft model capacity and fallback.
    pub aloft: AloftConfig,
}

impl MovingWorldConfig {
    /// Each library's default configuration under the given identities.
    pub fn new(
        surveillance_producer: surveillance_core::ProducerInstanceId,
        weather_producer: airmass_core::ProducerInstanceId,
    ) -> Self {
        Self {
            surveillance: EngineConfig::default(),
            surveillance_producer,
            weather: StoreConfig::default(),
            weather_producer,
            notices: NoticesConfig::default(),
            aloft: AloftConfig::default(),
        }
    }
}

/// What one [`crate::MovingWorld::advance`] changed, besides the traffic
/// deltas already passed to its callback.
#[derive(Clone, Debug, PartialEq)]
pub struct AdvanceReport {
    /// Airmass's own outcome: a new publication when visible weather
    /// changed, and the next instants at which it can.
    pub weather: AdvanceOutcome,
    /// Notices removed because they were cancelled or their validity ended.
    pub notices_expired: usize,
    /// Soundings and cloud layers removed because their validity ended.
    pub aloft_expired: usize,
    /// Product-bound soundings removed because Airmass no longer publishes
    /// their product as available.
    pub soundings_released: usize,
}
