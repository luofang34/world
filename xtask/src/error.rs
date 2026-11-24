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
    #[error("{file} sha256 {actual} does not match the recorded provenance {recorded}")]
    ChecksumMismatch {
        /// The committed coefficient file.
        file: &'static str,
        /// The checksum the provenance records.
        recorded: String,
        /// The committed file's checksum.
        actual: String,
    },
    /// The committed table differs from what the file generates.
    #[error("{path} is not what {file} generates; run `cargo run -p xtask -- {regenerate}`")]
    TableDiffers {
        /// The table.
        path: PathBuf,
        /// The committed coefficient file.
        file: &'static str,
        /// The task that rewrites the table.
        regenerate: &'static str,
    },
    /// The coefficient file does not have the expected arrays.
    #[error("{file}: {problem}")]
    Malformed {
        /// The committed coefficient file.
        file: &'static str,
        /// What is wrong with it.
        problem: String,
    },
}
