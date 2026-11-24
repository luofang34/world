//! Scratch memory for a synthesis, supplied by the caller.
//!
//! The synthesis keeps three rows of solid harmonics at a time (the
//! degree being summed and the two above it that its derivatives need),
//! plus square-root tables that turn every recursion and derivative
//! factor into products. So the buffer grows linearly with degree — about
//! 29 KB at degree 360 — and nothing is allocated per evaluation.

use crate::FieldError;

/// Length of the square-root tables for `max_degree`: the factors reach
/// √(2n + 5) at the top harmonic row, n = degree + 2.
const fn table_len(max_degree: usize) -> usize {
    2 * max_degree + 6
}

/// Entries in one harmonic row: orders 0..=degree + 2.
pub(crate) const fn row_len(max_degree: usize) -> usize {
    max_degree + 3
}

/// The `f64` count a [`Workspace`] for `max_degree` needs. A `const fn`,
/// so a fixed buffer can be declared without allocation:
/// `let mut buf = [0.0; workspace_len(12)];`.
pub const fn workspace_len(max_degree: usize) -> usize {
    2 * table_len(max_degree) + 6 * row_len(max_degree)
}

/// Scratch memory for syntheses up to a maximum degree, borrowed from a
/// caller-owned buffer and reused across evaluations.
#[derive(Debug)]
pub struct Workspace<'a> {
    pub(crate) max_degree: usize,
    pub(crate) sqrt: &'a [f64],
    pub(crate) inv_sqrt: &'a [f64],
    pub(crate) cos_rows: &'a mut [f64],
    pub(crate) sin_rows: &'a mut [f64],
}

impl<'a> Workspace<'a> {
    /// A workspace for degrees up to `max_degree` in `buffer`.
    ///
    /// # Errors
    ///
    /// [`FieldError::WorkspaceTooSmall`] if `buffer` is shorter than
    /// [`workspace_len`]`(max_degree)`.
    pub fn new(buffer: &'a mut [f64], max_degree: usize) -> Result<Self, FieldError> {
        fill_tables(buffer, max_degree)?;
        Self::prepared(buffer, max_degree)
    }

    /// The workspace over a buffer whose tables [`fill_tables`] already
    /// wrote.
    fn prepared(buffer: &'a mut [f64], max_degree: usize) -> Result<Self, FieldError> {
        let too_small = FieldError::WorkspaceTooSmall {
            required: workspace_len(max_degree),
            provided: buffer.len(),
        };
        let (t, r) = (table_len(max_degree), 3 * row_len(max_degree));
        let (sqrt, rest) = buffer.split_at_mut_checked(t).ok_or(too_small)?;
        let (inv_sqrt, rest) = rest.split_at_mut_checked(t).ok_or(too_small)?;
        let (cos_rows, rest) = rest.split_at_mut_checked(r).ok_or(too_small)?;
        let (sin_rows, _) = rest.split_at_mut_checked(r).ok_or(too_small)?;
        Ok(Self {
            max_degree,
            sqrt,
            inv_sqrt,
            cos_rows,
            sin_rows,
        })
    }

    /// The highest degree this workspace can synthesise.
    pub fn max_degree(&self) -> usize {
        self.max_degree
    }
}

/// Writes √k and 1/√k (1/√0 taken as 0, never used) at the head of
/// `buffer`.
fn fill_tables(buffer: &mut [f64], max_degree: usize) -> Result<(), FieldError> {
    let required = workspace_len(max_degree);
    if buffer.len() < required {
        return Err(FieldError::WorkspaceTooSmall {
            required,
            provided: buffer.len(),
        });
    }
    let t = table_len(max_degree);
    let (sqrt, rest) = buffer.split_at_mut(t.min(buffer.len()));
    for (k, slot) in sqrt.iter_mut().enumerate() {
        *slot = libm::sqrt(k as f64);
    }
    for (k, slot) in rest.iter_mut().take(t).enumerate() {
        *slot = if k == 0 {
            0.0
        } else {
            1.0 / libm::sqrt(k as f64)
        };
    }
    Ok(())
}

/// A heap-backed workspace buffer, filled once and lent out per
/// evaluation without recomputing its tables.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug)]
pub struct WorkspaceBuffer {
    buffer: alloc::vec::Vec<f64>,
    max_degree: usize,
}

#[cfg(feature = "alloc")]
impl WorkspaceBuffer {
    /// A buffer for degrees up to `max_degree`.
    pub fn new(max_degree: usize) -> Self {
        let mut buffer = alloc::vec![0.0; workspace_len(max_degree)];
        // The buffer is sized by `workspace_len`, so this cannot fail.
        fill_tables(&mut buffer, max_degree).ok();
        Self { buffer, max_degree }
    }

    /// The highest degree the buffer serves.
    pub fn max_degree(&self) -> usize {
        self.max_degree
    }

    /// A workspace over the buffer.
    ///
    /// # Errors
    ///
    /// None in practice: the buffer was sized for its degree. The
    /// `Result` keeps the length check in one place.
    pub fn workspace(&mut self) -> Result<Workspace<'_>, FieldError> {
        Workspace::prepared(&mut self.buffer, self.max_degree)
    }
}
