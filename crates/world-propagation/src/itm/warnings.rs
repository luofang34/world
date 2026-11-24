//! The reference code's warning flags, as a typed set.

/// Inputs or intermediate values near the edges of the model's validity.
/// The bit values are those of the reference code's `WARN__*` flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ItmWarnings(u16);

impl ItmWarnings {
    /// Transmitter height outside 1 to 1000 m.
    pub const TX_TERMINAL_HEIGHT: Self = Self(0x0001);
    /// Receiver height outside 1 to 1000 m.
    pub const RX_TERMINAL_HEIGHT: Self = Self(0x0002);
    /// Frequency outside 40 MHz to 10 GHz.
    pub const FREQUENCY: Self = Self(0x0004);
    /// Path longer than 1000 km.
    pub const PATH_DISTANCE_TOO_BIG_1: Self = Self(0x0008);
    /// Path longer than 2000 km: treat the result with care.
    pub const PATH_DISTANCE_TOO_BIG_2: Self = Self(0x0010);
    /// Path shorter than |h_e,tx − h_e,rx| / 0.2.
    pub const PATH_DISTANCE_TOO_SMALL_1: Self = Self(0x0020);
    /// Path shorter than 1 km: treat the result with care.
    pub const PATH_DISTANCE_TOO_SMALL_2: Self = Self(0x0040);
    /// Transmitter horizon angle above 200 mrad: small-angle geometry
    /// may break down.
    pub const TX_HORIZON_ANGLE: Self = Self(0x0080);
    /// Receiver horizon angle above 200 mrad.
    pub const RX_HORIZON_ANGLE: Self = Self(0x0100);
    /// Transmitter horizon nearer than 1/10 of the smooth-Earth horizon.
    pub const TX_HORIZON_DISTANCE_1: Self = Self(0x0200);
    /// Receiver horizon nearer than 1/10 of the smooth-Earth horizon.
    pub const RX_HORIZON_DISTANCE_1: Self = Self(0x0400);
    /// Transmitter horizon beyond 3 times the smooth-Earth horizon.
    pub const TX_HORIZON_DISTANCE_2: Self = Self(0x0800);
    /// Receiver horizon beyond 3 times the smooth-Earth horizon.
    pub const RX_HORIZON_DISTANCE_2: Self = Self(0x1000);
    /// A time, location or situation deviate beyond ±3.10.
    pub const EXTREME_VARIABILITIES: Self = Self(0x2000);
    /// Surface refractivity below 250 N-units.
    pub const SURFACE_REFRACTIVITY: Self = Self(0x4000);

    /// No warnings.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// The flags as the reference code's bit mask.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Whether no flag is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every flag of `other` is set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Sets the flags of `other`.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}
