//! Value-typed single-writer store for TFRs, surface closures and SUA activations.

use alloc::vec::Vec;
use world_core::{DecimalYear, GeodeticPosition};

use crate::{
    AirspaceKey, ClosureStatus, FacilityKey, NoticeError, NoticeId, RunwayKey, SuaActivationNotice,
    SurfaceClosureNotice, SurfaceElement, TfrNotice,
};

/// Capacity limits for a [`NoticesStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoticesConfig {
    /// Maximum stored TFR notices.
    pub max_tfrs: usize,
    /// Maximum stored aerodrome/runway surface notices.
    pub max_surface_closures: usize,
    /// Maximum stored SUA activation windows.
    pub max_sua_activations: usize,
}

impl Default for NoticesConfig {
    fn default() -> Self {
        Self {
            max_tfrs: 512,
            max_surface_closures: 2_048,
            max_sua_activations: 1_024,
        }
    }
}

/// Single-writer, value-typed operational notices store.
#[derive(Clone, Debug, Default)]
pub struct NoticesStore {
    config: NoticesConfig,
    revision: u64,
    tfrs: Vec<TfrNotice>,
    surface_closures: Vec<SurfaceClosureNotice>,
    sua_activations: Vec<SuaActivationNotice>,
}

impl NoticesStore {
    /// Creates an empty store with `config`.
    pub fn with_config(config: NoticesConfig) -> Self {
        Self {
            config,
            revision: 0,
            tfrs: Vec::new(),
            surface_closures: Vec::new(),
            sua_activations: Vec::new(),
        }
    }

    /// Monotonic revision incremented on every store mutation.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Inserts or replaces a validated [`TfrNotice`] by `notice.id`.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError`] if `notice.lateral` is malformed or capacity is exceeded.
    pub fn upsert_tfr(&mut self, notice: TfrNotice) -> Result<(), NoticeError> {
        notice.lateral.validate()?;
        if let Some(slot) = self.tfrs.iter_mut().find(|n| n.id == notice.id) {
            *slot = notice;
            self.bump();
            return Ok(());
        }
        if self.tfrs.len() >= self.config.max_tfrs {
            return Err(NoticeError::CapacityExceeded);
        }
        self.tfrs.push(notice);
        self.bump();
        Ok(())
    }

    /// Inserts or replaces a [`SurfaceClosureNotice`] by `notice.id`.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::CapacityExceeded`] when `max_surface_closures` would be exceeded.
    pub fn upsert_surface_closure(
        &mut self,
        notice: SurfaceClosureNotice,
    ) -> Result<(), NoticeError> {
        if let Some(slot) = self.surface_closures.iter_mut().find(|n| n.id == notice.id) {
            *slot = notice;
            self.bump();
            return Ok(());
        }
        if self.surface_closures.len() >= self.config.max_surface_closures {
            return Err(NoticeError::CapacityExceeded);
        }
        self.surface_closures.push(notice);
        self.bump();
        Ok(())
    }

    /// Inserts or replaces an [`SuaActivationNotice`] by `notice.id`.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::CapacityExceeded`] when `max_sua_activations` would be exceeded.
    pub fn upsert_sua_activation(
        &mut self,
        notice: SuaActivationNotice,
    ) -> Result<(), NoticeError> {
        if let Some(slot) = self.sua_activations.iter_mut().find(|n| n.id == notice.id) {
            *slot = notice;
            self.bump();
            return Ok(());
        }
        if self.sua_activations.len() >= self.config.max_sua_activations {
            return Err(NoticeError::CapacityExceeded);
        }
        self.sua_activations.push(notice);
        self.bump();
        Ok(())
    }

    /// Marks any notice matching `id` across TFRs, closures and SUA schedules as cancelled.
    pub fn cancel(&mut self, id: NoticeId) -> bool {
        let mut hit = false;
        for tfr in &mut self.tfrs {
            if tfr.id == id && !tfr.cancelled {
                tfr.cancelled = true;
                hit = true;
            }
        }
        for closure in &mut self.surface_closures {
            if closure.id == id && !closure.cancelled {
                closure.cancelled = true;
                hit = true;
            }
        }
        for sua in &mut self.sua_activations {
            if sua.id == id && !sua.cancelled {
                sua.cancelled = true;
                hit = true;
            }
        }
        if hit {
            self.bump();
        }
        hit
    }

    /// Removes cancelled notices and notices whose `valid.until` precedes `now`.
    pub fn prune_expired(&mut self, now: DecimalYear) -> usize {
        let before = self.tfrs.len() + self.surface_closures.len() + self.sua_activations.len();
        let keep = |cancelled: bool, until: DecimalYear| !cancelled && until.value() >= now.value();
        self.tfrs.retain(|n| keep(n.cancelled, n.valid.until));
        self.surface_closures
            .retain(|n| keep(n.cancelled, n.valid.until));
        self.sua_activations
            .retain(|n| keep(n.cancelled, n.valid.until));
        let after = self.tfrs.len() + self.surface_closures.len() + self.sua_activations.len();
        let removed = before.saturating_sub(after);
        if removed > 0 {
            self.bump();
        }
        removed
    }

    /// Active TFRs penetrated by `position` and `altitude_msl_m` at `at`.
    pub fn tfrs_containing(
        &self,
        position: GeodeticPosition,
        altitude_msl_m: f64,
        at: DecimalYear,
    ) -> Vec<&TfrNotice> {
        self.tfrs
            .iter()
            .filter(|tfr| tfr.contains(position, altitude_msl_m, at))
            .collect()
    }

    /// Active TFRs within `search_radius_m` horizontally of `position` at `at`.
    pub fn tfrs_near(
        &self,
        position: GeodeticPosition,
        search_radius_m: f64,
        at: DecimalYear,
    ) -> Vec<&TfrNotice> {
        self.tfrs
            .iter()
            .filter(|tfr| {
                tfr.is_active_at(at) && tfr.lateral.within_radius(position, search_radius_m)
            })
            .collect()
    }

    /// Whether `runway` (or the entire `facility` aerodrome) is closed at `at`.
    pub fn is_runway_closed(
        &self,
        facility: FacilityKey,
        runway: RunwayKey,
        at: DecimalYear,
    ) -> bool {
        self.surface_closures.iter().any(|n| {
            !n.cancelled
                && n.facility == facility
                && n.status == ClosureStatus::Closed
                && n.valid.contains(at)
                && (n.element == SurfaceElement::Aerodrome
                    || n.element == SurfaceElement::Runway(runway))
        })
    }

    /// Wait-free query of whether `airspace` has an active SUA schedule at `at`
    /// (and optionally overlapping `altitude_msl_m`).
    pub fn is_airspace_active(
        &self,
        airspace: AirspaceKey,
        altitude_msl_m: Option<f64>,
        at: DecimalYear,
    ) -> bool {
        self.sua_activations.iter().any(|n| {
            !n.cancelled
                && n.airspace == airspace
                && n.valid.contains(at)
                && altitude_msl_m.is_none_or(|alt| n.vertical.contains_msl_m(alt))
        })
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
