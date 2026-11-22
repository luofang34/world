//! ASTM F3411 / ASD-STAN Remote ID ingestion via the [`remoteid`] crate.

use remoteid::{IdType, Status, UasData};
use world_core::{GeodeticPosition, Ned, Uncertainty};

use crate::{
    AddressNamespace, Callsign, DeliveryPath, EmitterCategory, IntegrityQuality, ObservationSource,
    TrackKey, TrackObservation,
};

/// Ratio of the 95 % horizontal radius to 1-sigma for a circular normal error.
const HORIZONTAL_95_TO_SIGMA: f64 = 2.447_746_830_680_816;

impl TrackObservation {
    /// Builds an observation from a merged Remote ID record.
    ///
    /// Requires a Basic ID (keys the track in [`AddressNamespace::RemoteId`])
    /// and a Location message with a known position and geodetic (WGS-84
    /// ellipsoid) altitude; returns `None` otherwise. The F3411 horizontal
    /// accuracy code is the DO-260B NACp scale and is carried as `nac_p`.
    pub fn from_remote_id(
        uas: &UasData,
        delivery: DeliveryPath,
        observed_at_us: u64,
    ) -> Option<Self> {
        let id = uas
            .basic_id
            .iter()
            .flatten()
            .find(|id| id.id_type != IdType::None)?;
        let loc = uas.location.as_ref()?;
        let (lat, lon) = loc.position()?;
        let height = loc.altitude_geo?;

        let heading_rad = loc.direction.map(|d| f64::from(d).to_radians());
        let velocity_ned_m_s = match (loc.speed_horizontal, heading_rad) {
            (Some(speed), Some(track)) => Ned {
                north_m: f64::from(speed) * libm::cos(track),
                east_m: f64::from(speed) * libm::sin(track),
                down_m: -f64::from(loc.speed_vertical.unwrap_or(0.0)),
            },
            _ => Ned {
                north_m: 0.0,
                east_m: 0.0,
                down_m: -f64::from(loc.speed_vertical.unwrap_or(0.0)),
            },
        };

        let callsign = if id.id_type == IdType::CaaRegistrationId {
            id.uas_id_str().and_then(Callsign::parse)
        } else {
            None
        };

        Some(Self {
            key: TrackKey {
                namespace: AddressNamespace::RemoteId,
                address: fnv1a64(u8::from(id.id_type), &id.uas_id),
            },
            source: ObservationSource::RemoteId,
            delivery,
            observed_at_us,
            position: GeodeticPosition::wgs84(lat, lon, f64::from(height)),
            velocity_ned_m_s,
            heading_rad,
            callsign,
            category: EmitterCategory::Uav,
            on_ground: loc.status == Status::Ground,
            quality: IntegrityQuality {
                nic: 0,
                nac_p: loc.horizontal_accuracy.0,
                sil: 0,
                horizontal_1sigma_m: loc.horizontal_accuracy.meters().map(|m| Uncertainty {
                    one_sigma: f64::from(m) / HORIZONTAL_95_TO_SIGMA,
                }),
            },
        })
    }
}

/// Stable 64-bit address for a Remote ID identifier (FNV-1a over type + 20 ID bytes).
fn fnv1a64(id_type: u8, uas_id: &[u8; 20]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in core::iter::once(&id_type).chain(uas_id.iter()) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
