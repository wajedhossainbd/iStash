//! Stowaway core: moves heavy app data to external drives while apps keep working.
//! No UI here; the CLI and (later) the Tauri app call into this crate.

pub mod diskarb;
pub mod drives;
pub mod forecast;
pub mod fsx;
pub mod journal;
pub mod manifest;
pub mod metrics;
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

#[cfg(debug_assertions)]
#[doc(hidden)]
pub type StepHook = Option<Box<dyn Fn(&str)>>;

#[cfg(debug_assertions)]
thread_local! {
    static STEP_HOOK: std::cell::RefCell<StepHook> = const { std::cell::RefCell::new(None) };
}

/// Run `hook` at every named step on this thread (debug builds only): fault-injection tests detach drives,
/// fill them or change files at exact points. `None` removes it.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn set_step_hook(hook: StepHook) {
    STEP_HOOK.with(|h| *h.borrow_mut() = hook);
}

/// A named step: tests may kill the process here (`STOWAWAY_CRASH_AT`) or run a hook (debug builds only).
pub(crate) fn step(name: &str) {
    #[cfg(debug_assertions)]
    {
        if std::env::var("STOWAWAY_CRASH_AT").is_ok_and(|v| v == name) {
            // SAFETY: plain syscalls; SIGKILL ends the process like a crash or power loss would.
            unsafe { libc::kill(libc::getpid(), libc::SIGKILL) };
        }
        STEP_HOOK.with(|h| {
            if let Some(f) = h.borrow().as_ref() {
                f(name)
            }
        });
    }
    let _ = name;
}
