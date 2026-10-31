//! Compact ASCII keys referencing static facilities, runways and airspaces.

use core::fmt;

/// Errors returned when validating operational notice keys or geometries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeError {
    /// Identifier must be non-empty ASCII alphanumeric, hyphen or slash within length bounds.
    InvalidKey,
    /// Lateral circle radius or polygon vertex list is malformed.
    InvalidGeometry,
    /// Store capacity limit was reached.
    CapacityExceeded,
}

impl fmt::Display for NoticeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidKey => "notice identifier is empty or contains invalid ASCII characters",
            Self::InvalidGeometry => "notice lateral or vertical geometry is malformed",
            Self::CapacityExceeded => "notices store capacity limit exceeded",
        })
    }
}

impl core::error::Error for NoticeError {}

/// Compact 16-byte ASCII NOTAM or TFR identifier (for example, `"FDC 6/4321"`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoticeId([u8; 16]);

impl NoticeId {
    /// Parses a 1..=16 byte printable ASCII notice identifier, normalizing letters to uppercase.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::InvalidKey`] if empty, longer than 16 bytes, or non-ASCII.
    pub fn parse(text: &str) -> Result<Self, NoticeError> {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || bytes.len() > 16 {
            return Err(NoticeError::InvalidKey);
        }
        let mut buf = [b' '; 16];
        for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() && *src != b'/' && *src != b'-' && *src != b' ' {
                return Err(NoticeError::InvalidKey);
            }
            *dst = src.to_ascii_uppercase();
        }
        Ok(Self(buf))
    }

    /// Trimmed ASCII string view.
    pub fn as_str(&self) -> &str {
        trimmed_ascii(&self.0)
    }
}

/// Compact 8-byte ICAO/FAA facility identifier (for example, `"KDOV"` or `"33N"`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FacilityKey([u8; 8]);

impl FacilityKey {
    /// Parses a 1..=8 byte ASCII alphanumeric facility identifier.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::InvalidKey`] if empty, > 8 bytes, or non-alphanumeric.
    pub fn parse(text: &str) -> Result<Self, NoticeError> {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || bytes.len() > 8 {
            return Err(NoticeError::InvalidKey);
        }
        let mut buf = [b' '; 8];
        for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() {
                return Err(NoticeError::InvalidKey);
            }
            *dst = src.to_ascii_uppercase();
        }
        Ok(Self(buf))
    }

    /// Trimmed ASCII string view.
    pub fn as_str(&self) -> &str {
        trimmed_ascii(&self.0)
    }
}

/// Compact 8-byte runway or runway-end designator (for example, `"01/19"` or `"19R"`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunwayKey([u8; 8]);

impl RunwayKey {
    /// Parses a 1..=8 byte ASCII runway designator (`0-9`, `L`, `C`, `R`, `W`, `/`).
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::InvalidKey`] if empty, > 8 bytes, or contains invalid bytes.
    pub fn parse(text: &str) -> Result<Self, NoticeError> {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || bytes.len() > 8 {
            return Err(NoticeError::InvalidKey);
        }
        let mut buf = [b' '; 8];
        for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() && *src != b'/' {
                return Err(NoticeError::InvalidKey);
            }
            *dst = src.to_ascii_uppercase();
        }
        Ok(Self(buf))
    }

    /// Trimmed ASCII string view.
    pub fn as_str(&self) -> &str {
        trimmed_ascii(&self.0)
    }
}

/// Compact 16-byte Special Use Airspace designator (for example, `"R-4006"` or `"DOVER MOA"`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirspaceKey([u8; 16]);

impl AirspaceKey {
    /// Parses a 1..=16 byte ASCII airspace designator.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::InvalidKey`] if empty, > 16 bytes, or contains invalid bytes.
    pub fn parse(text: &str) -> Result<Self, NoticeError> {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || bytes.len() > 16 {
            return Err(NoticeError::InvalidKey);
        }
        let mut buf = [b' '; 16];
        for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() && *src != b'-' && *src != b' ' && *src != b'/' {
                return Err(NoticeError::InvalidKey);
            }
            *dst = src.to_ascii_uppercase();
        }
        Ok(Self(buf))
    }

    /// Trimmed ASCII string view.
    pub fn as_str(&self) -> &str {
        trimmed_ascii(&self.0)
    }
}

fn trimmed_ascii(bytes: &[u8]) -> &str {
    let mut end = bytes.len();
    while end > 0 {
        if bytes.get(end - 1) == Some(&b' ') {
            end -= 1;
        } else {
            break;
        }
    }
    core::str::from_utf8(bytes.get(..end).unwrap_or(&[])).unwrap_or_default()
}

impl fmt::Debug for NoticeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoticeId({:?})", self.as_str())
    }
}
impl fmt::Display for NoticeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Debug for FacilityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FacilityKey({:?})", self.as_str())
    }
}
impl fmt::Display for FacilityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Debug for RunwayKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RunwayKey({:?})", self.as_str())
    }
}
impl fmt::Display for RunwayKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Debug for AirspaceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AirspaceKey({:?})", self.as_str())
    }
}
impl fmt::Display for AirspaceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
