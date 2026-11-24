//! Reading the committed files a task compares.

use std::fmt::Write as _;
use std::path::Path;

use crate::error::XtaskError;

/// The bytes of `relative` under the repository root.
pub(crate) fn read(root: &Path, relative: &str) -> Result<Vec<u8>, XtaskError> {
    let path = root.join(relative);
    std::fs::read(&path).map_err(|source| XtaskError::Io {
        action: "reading",
        path,
        source,
    })
}

/// The text of `relative` under the repository root.
pub(crate) fn read_text(root: &Path, relative: &str) -> Result<String, XtaskError> {
    let bytes = read(root, relative)?;
    String::from_utf8(bytes).map_err(|_| XtaskError::NotText {
        path: root.join(relative),
    })
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        // Writing to a String cannot fail.
        write!(out, "{byte:02x}").ok();
        out
    })
}
