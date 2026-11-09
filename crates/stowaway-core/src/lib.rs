//! Stowaway core: moves heavy app data to external drives while apps keep working.
//! No UI here; the CLI and (later) the Tauri app call into this crate.

pub mod diskarb;
pub mod drives;
pub mod fsx;
pub mod journal;
pub mod mover;
pub mod procs;
pub mod rules;
pub mod scan;

use std::fmt;

/// A refusal or failure to show the user as is. Nothing was changed unless the message says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fail(pub String);

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Fail {}

impl From<std::io::Error> for Fail {
    fn from(e: std::io::Error) -> Self {
        Fail(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Fail>;

pub(crate) fn fail<T>(msg: impl Into<String>) -> Result<T> {
    Err(Fail(msg.into()))
}
