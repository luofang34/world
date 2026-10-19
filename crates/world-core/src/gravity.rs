//! The Earth's normal gravity: the gravity of the datum's level ellipsoid.

use libm::{sin, sqrt};

use crate::{GeodeticPosition, HorizontalDatum, PositionError};

/// The WGS-84 normal gravity field (NIMA TR8350.2, table 3.4).
struct NormalGravityField {
    /// At the equator, m/s².
    equator_mps2: f64,
    /// Somigliana's constant k = b·γp / (a·γe) − 1.
    somigliana_k: f64,
    /// m = ω²a²b / GM.
    m: f64,
}

const WGS84_NORMAL_GRAVITY: NormalGravityField = NormalGravityField {
    equator_mps2: 9.780_325_335_9,
    somigliana_k: 0.001_931_852_652_41,
    m: 0.003_449_786_506_84,
};

/// Normal gravity at `position`, m/s², along the ellipsoid normal: the
/// Somigliana closed form on the ellipsoid, corrected for height to second
/// order (TR8350.2 eq. 4-1 and 4-3). Gravity anomalies and deflections
/// of the vertical are not modelled.
///
/// # Errors
///
/// [`PositionError::NotWellFormed`] for a malformed position.
pub fn normal_gravity(position: GeodeticPosition) -> Result<f64, PositionError> {
    if !position.is_well_formed() {
        return Err(PositionError::NotWellFormed);
    }
    let field = match position.datum {
        HorizontalDatum::Wgs84 => WGS84_NORMAL_GRAVITY,
    };
    let ellipsoid = position.datum.ellipsoid();
    let (a, f) = (ellipsoid.semi_major_m, ellipsoid.flattening);
    let e2 = ellipsoid.eccentricity_squared();
    let sin_lat = sin(position.latitude_deg.to_radians());
    let s2 = sin_lat * sin_lat;
    let on_ellipsoid = field.equator_mps2 * (1.0 + field.somigliana_k * s2) / sqrt(1.0 - e2 * s2);
    let h = position.ellipsoidal_height_m;
    Ok(on_ellipsoid
        * (1.0 - 2.0 / a * (1.0 + f + field.m - 2.0 * f * s2) * h + 3.0 / (a * a) * h * h))
}

#[cfg(test)]
mod tests;
