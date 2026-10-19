//! Why a task failed.

use std::path::PathBuf;

/// Why a task failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum XtaskError {
    /// No task, or an unknown one.
    #[error("{0}")]
    Usage(&'static str),
    /// A file could not be read or written.
    #[error("{action} {path}: {source}")]
    Io {
        /// "reading" or "writing".
        action: &'static str,
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: std::io::Error,
    },
    /// The coefficient file is not UTF-8 text.
    #[error("{path} is not UTF-8 text")]
    NotText {
        /// The file.
        path: PathBuf,
    },
    /// The provenance names no checksum.
    #[error("{path} records no source_sha256")]
    NoRecordedChecksum {
        /// The file that should record it.
        path: PathBuf,
    },
    /// The coefficient file is not the one the provenance names.
    #[error("WMM.COF sha256 {actual} does not match the recorded provenance {recorded}")]
    ChecksumMismatch {
        /// The checksum the provenance records.
        recorded: String,
        /// The committed file's checksum.
        actual: String,
    },
    /// The committed table differs from what the file generates.
    #[error("{path} is not what WMM.COF generates; run `cargo run -p xtask -- gen-wmm`")]
    TableDiffers {
        /// The table.
        path: PathBuf,
    },
}
