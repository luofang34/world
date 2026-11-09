//! Target identity, address namespaces, callsigns and emitter categories.

use core::fmt;

/// Engine-assigned stable target identifier within a [`crate::SurveillanceStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u64);

/// Address namespace distinguishing how a target address was assigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AddressNamespace {
    /// Globally unique 24-bit ICAO aircraft address.
    Icao24,
    /// Non-ICAO self-assigned or anonymous ADS-B / UAT address.
    AdsbNonIcao,
    /// TIS-B ground-station track file identifier.
    TisbTrackFile,
    /// FLARM radio device or stealth identifier.
    Flarm,
    /// ASTM F3411 / ASD-STAN Remote ID UAS serial or session identifier.
    RemoteId,
    /// Tactical or air-defense C2 network track number.
    C2Track,
    /// Airport surface vehicle squitter or fleet identifier.
    SurfaceVehicle,
    /// Fixed ground obstacle or calibration beacon.
    FixedBeacon,
    /// Deterministic simulator entity identifier.
    Simulated,
}

/// Namespaced external target key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackKey {
    /// Address namespace.
    pub namespace: AddressNamespace,
    /// Numeric address or track number within `namespace`.
    pub address: u64,
}

/// Compact 8-character ASCII flight identification / callsign.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Callsign([u8; 8]);

impl Callsign {
    /// Parses up to 8 ASCII graphic characters (or spaces), trimming trailing whitespace.
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || bytes.len() > 8 {
            return None;
        }
        let mut buf = [b' '; 8];
        for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() && *src != b'-' && *src != b'_' {
                return None;
            }
            *dst = src.to_ascii_uppercase();
        }
        Some(Self(buf))
    }

    /// Trimmed ASCII string view of the callsign.
    pub fn as_str(&self) -> &str {
        let mut end = 8usize;
        while end > 0 {
            if self.0.get(end - 1) == Some(&b' ') {
                end -= 1;
            } else {
                break;
            }
        }
        core::str::from_utf8(self.0.get(..end).unwrap_or(&[])).unwrap_or_default()
    }
}

impl fmt::Debug for Callsign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Callsign({:?})", self.as_str())
    }
}

impl fmt::Display for Callsign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// DO-260B / GDL-90 emitter category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmitterCategory {
    /// No category information reported.
    #[default]
    Unknown,
    /// Light aircraft (< 15,500 lb).
    Light,
    /// Small aircraft (15,500..75,000 lb).
    Small,
    /// Large aircraft (75,000..300,000 lb).
    Large,
    /// High-vortex large aircraft (e.g. B757).
    HighVortexLarge,
    /// Heavy aircraft (>= 300,000 lb).
    Heavy,
    /// High-performance (> 5 g and > 400 kt).
    HighPerformance,
    /// Rotorcraft / helicopter.
    Rotorcraft,
    /// Glider / sailplane.
    Glider,
    /// Lighter-than-air airship or balloon.
    LighterThanAir,
    /// Parachutist / skydiver.
    Parachutist,
    /// Ultralight / hang-glider / paraglider.
    Ultralight,
    /// Unmanned aerial vehicle / drone.
    Uav,
    /// Space or transatmospheric vehicle.
    SpaceTransatmospheric,
    /// Surface emergency vehicle.
    SurfaceEmergency,
    /// Surface service vehicle.
    SurfaceService,
    /// Point obstacle or tethered balloon.
    PointObstacle,
}
