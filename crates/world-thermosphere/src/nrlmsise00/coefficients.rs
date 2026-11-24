//! The model's fitted parameters, as published.

mod lower;
mod upper;

pub(crate) use lower::{PAVGM, PDM, PMA, PTL, PTM};
pub(crate) use upper::{PD, PDL, PS, PT};
