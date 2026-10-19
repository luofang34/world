//! What a model's answer comes from, and for which dates it holds.

use crate::DecimalYear;

/// A published dataset, by name and edition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatasetId {
    /// The dataset's name, as its publisher gives it.
    pub name: &'static str,
    /// The edition or release, as its publisher gives it.
    pub edition: &'static str,
}

/// Where a dataset's values came from, so an answer can be traced to the
/// exact published file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The dataset.
    pub dataset: DatasetId,
    /// Who publishes it.
    pub publisher: &'static str,
    /// Where the published file is obtained.
    pub source_url: &'static str,
    /// The published file the values were taken from.
    pub source_file: &'static str,
    /// The SHA-256 of that file, lowercase hex.
    pub source_sha256: &'static str,
    /// The terms the dataset is published under.
    pub terms: &'static str,
}

/// The dates a dataset's answers hold for, inclusive at both ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidityWindow {
    /// First valid date.
    pub from: DecimalYear,
    /// Last valid date.
    pub until: DecimalYear,
}

impl ValidityWindow {
    /// Whether `at` falls inside the window. A NaN date never does.
    pub fn contains(&self, at: DecimalYear) -> bool {
        (self.from.value()..=self.until.value()).contains(&at.value())
    }
}

#[cfg(test)]
mod tests;
