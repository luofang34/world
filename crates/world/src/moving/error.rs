//! Why a moving world refused to be built or advanced.

use airmass_core::{StoreError, WeatherProductId};
use surveillance_core::{EngineConfigError, SnapshotPublicationError};
use thiserror::Error;
use world_aloft::AloftError;

/// Failure building, feeding or advancing a [`crate::MovingWorld`].
///
/// Each library's own error is kept as the source, unchanged.
#[derive(Clone, Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum MovingWorldError {
    /// Surveillance rejected its engine configuration.
    #[error("surveillance engine configuration is invalid")]
    SurveillanceConfig(#[source] EngineConfigError),
    /// Airmass rejected its configuration or an ingress.
    #[error("the weather store refused the operation")]
    Weather(#[source] StoreError),
    /// The engine could not publish a track state while advancing.
    #[error("the surveillance engine could not publish a track state")]
    TrackPublication(#[source] SnapshotPublicationError),
    /// The monotonic clock was stepped back.
    #[error("monotonic clock went back from {previous_micros} us to {supplied_micros} us")]
    MonotonicRegression {
        /// The reading the world was last advanced to.
        previous_micros: u64,
        /// The earlier reading supplied.
        supplied_micros: u64,
    },
    /// A sounding was bound to a product Airmass does not publish as
    /// available.
    #[error("weather product {product_id:?} is not available to bind a sounding to")]
    ProductNotAvailable {
        /// The product named.
        product_id: WeatherProductId,
    },
    /// The aloft model did not take a sounding.
    #[error("the aloft model did not take sounding {sounding_id}")]
    Sounding {
        /// The sounding offered.
        sounding_id: u64,
        /// The aloft model's reason.
        #[source]
        source: AloftError,
    },
}
