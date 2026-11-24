//! The canonical byte content of a model, for hashing by content.
//!
//! The crate fixes no hash function: a consumer feeds the bytes to its own.
//! The content is the concatenation of every slice handed to the visitor;
//! how it is cut into slices is unspecified and never significant.
//!
//! Encoding: numbers are little-endian (`f64` as its IEEE-754 bit pattern,
//! so `-0.0` and `0.0` differ and a NaN is its own bits; `usize` as `u64`),
//! every sequence is preceded by its length, every enum and `Option` by a
//! one-byte tag, and a model starts with a one-byte kind tag. Storage type
//! is not content: a `Vec`, an array and a slice holding the same numbers
//! give the same bytes.

use world_core::{DecimalYear, Ellipsoid, Provenance, ValidityWindow};

use crate::{HarmonicCoefficients, Truncation};

/// A model that can state its canonical content as bytes.
pub trait CanonicalContent {
    /// Calls `f` with consecutive slices whose concatenation is the
    /// model's canonical content: everything that influences a sample.
    fn visit_content(&self, f: &mut dyn FnMut(&[u8]));
}

/// Kind tags that start each model's content, so two different kinds of
/// model never share a feed.
pub(crate) mod kind {
    pub(crate) const GRAVITY: u8 = 1;
    pub(crate) const MAGNETIC: u8 = 2;
    #[cfg(feature = "alloc")]
    pub(crate) const MAGNETIC_SERIES: u8 = 3;
    pub(crate) const VECTOR_GRID: u8 = 4;
    pub(crate) const SCALAR_GRID: u8 = 5;
}

pub(crate) fn feed_u8(f: &mut dyn FnMut(&[u8]), v: u8) {
    f(&[v]);
}

pub(crate) fn feed_usize(f: &mut dyn FnMut(&[u8]), v: usize) {
    f(&(v as u64).to_le_bytes());
}

pub(crate) fn feed_f64(f: &mut dyn FnMut(&[u8]), v: f64) {
    f(&v.to_bits().to_le_bytes());
}

pub(crate) fn feed_str(f: &mut dyn FnMut(&[u8]), s: &str) {
    feed_usize(f, s.len());
    f(s.as_bytes());
}

/// A length-prefixed `f64` run, batched so a large grid is not one call
/// per number.
pub(crate) fn feed_f64s(f: &mut dyn FnMut(&[u8]), values: &[f64]) {
    feed_usize(f, values.len());
    let mut buffer = [0_u8; 256];
    for chunk in values.chunks(buffer.len() / 8) {
        for (slot, v) in buffer.chunks_exact_mut(8).zip(chunk) {
            slot.copy_from_slice(&v.to_bits().to_le_bytes());
        }
        f(buffer.get(..8 * chunk.len()).unwrap_or(&[]));
    }
}

pub(crate) fn feed_year(f: &mut dyn FnMut(&[u8]), year: DecimalYear) {
    feed_f64(f, year.0);
}

pub(crate) fn feed_validity(f: &mut dyn FnMut(&[u8]), w: ValidityWindow) {
    feed_year(f, w.from);
    feed_year(f, w.until);
}

pub(crate) fn feed_ellipsoid(f: &mut dyn FnMut(&[u8]), e: Ellipsoid) {
    feed_f64(f, e.semi_major_m);
    feed_f64(f, e.flattening);
}

pub(crate) fn feed_truncation(f: &mut dyn FnMut(&[u8]), t: Truncation) {
    feed_usize(f, t.min_degree);
    feed_usize(f, t.degree);
    feed_usize(f, t.order);
}

pub(crate) fn feed_provenance(f: &mut dyn FnMut(&[u8]), p: Option<Provenance>) {
    let Some(p) = p else {
        return feed_u8(f, 0);
    };
    feed_u8(f, 1);
    for s in [
        p.dataset.name,
        p.dataset.edition,
        p.publisher,
        p.source_url,
        p.source_file,
        p.source_sha256,
        p.terms,
    ] {
        feed_str(f, s);
    }
}

impl<B: AsRef<[f64]>> CanonicalContent for HarmonicCoefficients<B> {
    fn visit_content(&self, f: &mut dyn FnMut(&[u8])) {
        feed_usize(f, self.degree());
        feed_f64s(f, self.cosine());
        feed_f64s(f, self.sine());
    }
}

#[cfg(test)]
mod tests;
