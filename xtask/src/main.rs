//! Repository maintenance tasks.
//!
//! - `gen-wmm`: write `world-geomag`'s coefficient table from the
//!   committed `WMM.COF`.
//! - `check-wmm`: fail unless the committed table is exactly what the
//!   committed `WMM.COF` generates, and that file is the one the
//!   `WMM2025` provenance names by checksum.

mod error;
mod wmm;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use error::XtaskError;

const USAGE: &str = "usage: cargo run -p xtask -- <gen-wmm | check-wmm>";

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let outcome = match task.as_deref() {
        Some("gen-wmm") => wmm::write(&repo_root()).map(|()| "wrote the WMM2025 coefficient table"),
        Some("check-wmm") => wmm::check(&repo_root())
            .map(|()| "WMM2025 coefficients reproduce from the recorded WMM.COF"),
        _ => Err(XtaskError::Usage(USAGE)),
    };
    let (mut out, message, code): (Box<dyn Write>, String, ExitCode) = match outcome {
        Ok(done) => (
            Box::new(std::io::stdout()),
            done.to_owned(),
            ExitCode::SUCCESS,
        ),
        Err(error) => (
            Box::new(std::io::stderr()),
            error.to_string(),
            ExitCode::FAILURE,
        ),
    };
    // Nothing is left to report a failed write to.
    writeln!(out, "{message}").ok();
    code
}

/// The workspace root: the parent of this crate's manifest directory.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map_or(manifest.clone(), PathBuf::from)
}
