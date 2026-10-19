//! Reference ellipsoids and Earth-centred, Earth-fixed coordinates.

use libm::{atan2, cbrt, cos, sin, sqrt};

/// A reference ellipsoid, by its defining semi-major axis and flattening.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoid {
    /// Semi-major (equatorial) axis, meters.
    pub semi_major_m: f64,
    /// Flattening, `(a - b) / a`.
    pub flattening: f64,
}

/// The WGS-84 ellipsoid (NIMA TR8350.2, table 3.1).
pub const WGS84: Ellipsoid = Ellipsoid {
    semi_major_m: 6_378_137.0,
    flattening: 1.0 / 298.257_223_563,
};

/// Distance from the polar axis, meters, inside which a point is taken to
/// be on it. A micrometre moves the latitude by under 1e-13 degrees.
const POLAR_AXIS_M: f64 = 1e-6;

/// Earth-centred, Earth-fixed Cartesian coordinates, meters: x toward the
/// equator at the prime meridian, z toward the north pole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ecef {
    /// Toward latitude 0, longitude 0.
    pub x_m: f64,
    /// Toward latitude 0, longitude 90° east.
    pub y_m: f64,
    /// Toward the north pole.
    pub z_m: f64,
}

impl Ellipsoid {
    /// Semi-minor (polar) axis, meters.
    pub fn semi_minor_m(&self) -> f64 {
        self.semi_major_m * (1.0 - self.flattening)
    }

    /// First eccentricity squared.
    pub fn eccentricity_squared(&self) -> f64 {
        self.flattening * (2.0 - self.flattening)
    }

    /// The ECEF point at geodetic latitude, longitude (degrees) and height
    /// above this ellipsoid (meters).
    pub fn to_ecef(&self, latitude_deg: f64, longitude_deg: f64, height_m: f64) -> Ecef {
        let (lat, lon) = (latitude_deg.to_radians(), longitude_deg.to_radians());
        let e2 = self.eccentricity_squared();
        let prime_vertical = self.semi_major_m / sqrt(1.0 - e2 * sin(lat) * sin(lat));
        Ecef {
            x_m: (prime_vertical + height_m) * cos(lat) * cos(lon),
            y_m: (prime_vertical + height_m) * cos(lat) * sin(lon),
            z_m: (prime_vertical * (1.0 - e2) + height_m) * sin(lat),
        }
    }

    /// Geodetic latitude, longitude (degrees) and height (meters) of an
    /// ECEF point, in closed form (Heikkinen 1982), so the result is exact
    /// to rounding without iteration. Undefined within about 43 km of the
    /// Earth's centre.
    pub fn from_ecef(&self, point: Ecef) -> (f64, f64, f64) {
        let a = self.semi_major_m;
        let b = self.semi_minor_m();
        let e2 = self.eccentricity_squared();
        let ep2 = (a * a - b * b) / (b * b);
        let Ecef {
            x_m: x,
            y_m: y,
            z_m: z,
        } = point;
        let p = sqrt(x * x + y * y);
        if p < POLAR_AXIS_M {
            // On the polar axis the latitude is ±90° and the height is
            // the distance past the pole; the general form's terms
            // vanish there and rounding drives its square roots negative.
            let latitude = if z >= 0.0 { 90.0 } else { -90.0 };
            return (latitude, atan2(y, x).to_degrees(), z.abs() - b);
        }
        let f = 54.0 * b * b * z * z;
        let g = p * p + (1.0 - e2) * z * z - e2 * (a * a - b * b);
        let c = e2 * e2 * f * p * p / (g * g * g);
        let s = cbrt(1.0 + c + sqrt(c * c + 2.0 * c));
        let k = s + 1.0 + 1.0 / s;
        let big_p = f / (3.0 * k * k * g * g);
        let q = sqrt(1.0 + 2.0 * e2 * e2 * big_p);
        let r0 = -(big_p * e2 * p) / (1.0 + q)
            + sqrt(
                (0.5 * a * a * (1.0 + 1.0 / q)
                    - big_p * (1.0 - e2) * z * z / (q * (1.0 + q))
                    - 0.5 * big_p * p * p)
                    .max(0.0),
            );
        let u = sqrt((p - e2 * r0) * (p - e2 * r0) + z * z);
        let v = sqrt((p - e2 * r0) * (p - e2 * r0) + (1.0 - e2) * z * z);
        let z0 = b * b * z / (a * v);
        let height = u * (1.0 - b * b / (a * v));
        let latitude = atan2(z + ep2 * z0, p);
        let longitude = atan2(y, x);
        (latitude.to_degrees(), longitude.to_degrees(), height)
    }
}

#[cfg(test)]
mod tests;
