//! The moving operational world: Surveillance's engine, Airmass's store, the
//! notices store and the aloft model, driven by one clock.

use alloc::vec::Vec;

use airmass_core::{AcceptOutcome, WeatherIngress, WeatherProductId, WeatherStore};
use surveillance_core::{SurveillanceEngine, TrackDelta};
use world_aloft::{AloftModel, SoundingProfile};
use world_core::WorldInstant;
use world_notices::NoticesStore;

use crate::clock::evaluation_time;

mod config;
mod error;

pub use config::{AdvanceReport, MovingWorldConfig};
pub use error::MovingWorldError;

/// One knowledge state of the moving world.
///
/// Traffic state is the [`SurveillanceEngine`] and weather state the
/// [`WeatherStore`]; a host feeds each with that library's own ingress type
/// and reads each through that library's own snapshots, whose fields carry
/// the library's own provenance. Nothing here copies either.
///
/// A world owns its members by value and reaches no global state, so a
/// simulator holds one for physical truth and another for each vehicle's
/// delayed or range-limited belief. The engine and the store are
/// single-writer and deliberately not `Clone`: a belief is fed its own
/// observations and ingress, never forked from the truth.
#[derive(Debug)]
pub struct MovingWorld {
    /// Traffic state: the Surveillance engine.
    pub surveillance: SurveillanceEngine,
    /// Weather state: the Airmass store.
    pub weather: WeatherStore,
    /// Active TFRs, aerodrome/runway closures and SUA activations.
    pub notices: NoticesStore,
    /// Physical model of the air: soundings and cloud layers.
    pub aloft: AloftModel,
    product_soundings: Vec<ProductSounding>,
    now: Option<WorldInstant>,
}

/// A sounding a host decoded from one Airmass product.
#[derive(Clone, Debug)]
struct ProductSounding {
    sounding_id: u64,
    product_id: WeatherProductId,
}

impl MovingWorld {
    /// Creates an empty world.
    ///
    /// # Errors
    ///
    /// [`MovingWorldError`] when the engine or the store rejects its
    /// configuration.
    pub fn new(config: MovingWorldConfig) -> Result<Self, MovingWorldError> {
        Ok(Self {
            surveillance: SurveillanceEngine::new(
                config.surveillance,
                config.surveillance_producer,
            )
            .map_err(MovingWorldError::SurveillanceConfig)?,
            weather: WeatherStore::new(config.weather, config.weather_producer)
                .map_err(MovingWorldError::Weather)?,
            notices: NoticesStore::with_config(config.notices),
            aloft: AloftModel::with_config(config.aloft),
            product_soundings: Vec::new(),
            now: None,
        })
    }

    /// The instant the world was last advanced to.
    pub const fn now(&self) -> Option<WorldInstant> {
        self.now
    }

    /// Advances every member to `now`, passing each traffic delta to
    /// `on_track_delta` before returning.
    ///
    /// The two readings of `now` drive different things and neither is
    /// derived from the other. Its monotonic reading ages tracks
    /// (Surveillance) and bounds a weather product's maximum age (Airmass).
    /// Its UTC reading decides weather and notice validity and the date the
    /// aloft model is pruned at. The monotonic reading must not go back;
    /// Airmass also refuses a UTC reading earlier than the last one it
    /// accepted, and that refusal is returned unchanged.
    ///
    /// The weather store is advanced first because it is the member that can
    /// refuse a time, so a refused instant leaves every member where it was.
    /// Advancing twice to one instant is harmless.
    ///
    /// # Errors
    ///
    /// [`MovingWorldError::MonotonicRegression`] before anything advances,
    /// [`MovingWorldError::Weather`] when Airmass refuses the instant, and
    /// [`MovingWorldError::TrackPublication`] when the engine cannot publish
    /// a track state.
    pub fn advance(
        &mut self,
        now: WorldInstant,
        on_track_delta: impl FnMut(&TrackDelta),
    ) -> Result<AdvanceReport, MovingWorldError> {
        if let Some(previous) = self.now
            && now.monotonic_micros < previous.monotonic_micros
        {
            return Err(MovingWorldError::MonotonicRegression {
                previous_micros: previous.monotonic_micros,
                supplied_micros: now.monotonic_micros,
            });
        }
        let weather = self
            .weather
            .advance_time(evaluation_time(now))
            .map_err(MovingWorldError::Weather)?;
        self.surveillance
            .advance_time(now.monotonic_micros, on_track_delta)
            .map_err(MovingWorldError::TrackPublication)?;
        let date = now.decimal_year();
        let notices_expired = self.notices.prune_expired(date);
        let aloft_expired = self.aloft.prune_expired(date);
        let soundings_released = self.release_soundings();
        self.now = Some(now);
        Ok(AdvanceReport {
            weather,
            notices_expired,
            aloft_expired,
            soundings_released,
        })
    }

    /// Accepts one weather ingress at `now`, stated in Airmass's own time.
    ///
    /// # Errors
    ///
    /// [`MovingWorldError::Weather`] with Airmass's own refusal.
    pub fn accept_weather(
        &mut self,
        ingress: WeatherIngress,
        now: WorldInstant,
    ) -> Result<AcceptOutcome, MovingWorldError> {
        self.weather
            .accept(ingress, evaluation_time(now))
            .map_err(MovingWorldError::Weather)
    }

    /// Takes a sounding a host decoded from the Airmass product `product_id`
    /// into the aloft model, for as long as Airmass publishes that product.
    ///
    /// Airmass carries winds and temperatures aloft as a text report, not as
    /// typed levels, so a host decodes the levels. This binding keeps the
    /// decoded sounding from becoming a second weather state: its source is
    /// the product's own provenance in the weather snapshot, and
    /// [`MovingWorld::advance`] removes it once Airmass no longer publishes
    /// the product as available.
    ///
    /// # Errors
    ///
    /// [`MovingWorldError::ProductNotAvailable`] when the weather snapshot
    /// does not publish `product_id` as available, and
    /// [`MovingWorldError::Sounding`] when the aloft model is full.
    pub fn bind_sounding(
        &mut self,
        product_id: &WeatherProductId,
        profile: SoundingProfile,
    ) -> Result<(), MovingWorldError> {
        if !self.publishes(product_id) {
            return Err(MovingWorldError::ProductNotAvailable {
                product_id: product_id.clone(),
            });
        }
        let sounding_id = profile.id();
        self.aloft
            .upsert_sounding(profile)
            .map_err(|source| MovingWorldError::Sounding {
                sounding_id,
                source,
            })?;
        self.product_soundings
            .retain(|bound| bound.sounding_id != sounding_id);
        self.product_soundings.push(ProductSounding {
            sounding_id,
            product_id: product_id.clone(),
        });
        Ok(())
    }

    /// The Airmass product the sounding `sounding_id` was decoded from, when
    /// it was bound to one. The product's provenance, revision and validity
    /// are read from the weather snapshot under this identity.
    pub fn sounding_product(&self, sounding_id: u64) -> Option<&WeatherProductId> {
        self.product_soundings
            .iter()
            .find(|bound| bound.sounding_id == sounding_id)
            .map(|bound| &bound.product_id)
    }

    fn publishes(&self, product_id: &WeatherProductId) -> bool {
        self.weather
            .capture()
            .snapshot()
            .product(product_id)
            .is_some_and(|product| product.state().available().is_some())
    }

    fn release_soundings(&mut self) -> usize {
        if self.product_soundings.is_empty() {
            return 0;
        }
        let published = self.weather.capture();
        let snapshot = published.snapshot();
        let (kept, released): (Vec<_>, Vec<_>) =
            self.product_soundings.drain(..).partition(|bound| {
                snapshot
                    .product(&bound.product_id)
                    .is_some_and(|product| product.state().available().is_some())
            });
        self.product_soundings = kept;
        // A sounding the aloft model already dropped is still unbound here.
        released
            .iter()
            .filter(|bound| self.aloft.remove_sounding(bound.sounding_id))
            .count()
    }
}

#[cfg(test)]
mod tests;
