//! Manages process-wide exit requests and coordination.
//!
//! This is an interesting cross-cutting problem where global state is the only
//! practical solution. Exiting is a behavior of the global process and is not a
//! per-fiber activity.

use std::sync::atomic::{AtomicUsize, Ordering};

const NONE_VALUE: usize = usize::max_value();
static EXIT_CODE: AtomicUsize = AtomicUsize::new(NONE_VALUE);

/// Get the current exit code for the process. If no exit has been requested,
/// then `None` will be returned.
pub fn get() -> Option<i32> {
    match EXIT_CODE.load(Ordering::SeqCst) {
        NONE_VALUE => None,
        code => Some(code as i32),
    }
}

/// Request the process to exit with the given exit code.
pub fn set(code: i32) {
    let mut code = code as usize;

    // Can't use this exit code as it means "none".
    if code == NONE_VALUE {
        code = NONE_VALUE - 1;
    }

    log::debug!("exit requested with code {}", code);

    match EXIT_CODE.compare_and_swap(NONE_VALUE, code, Ordering::SeqCst) {
        // Exit code set.
        NONE_VALUE => {},

        // Upgrade a zero exit code to a nonzero one.
        0 => {
            EXIT_CODE.store(code, Ordering::SeqCst);
        },

        // Do not change an existing nonzero code if already exiting.
        _ => {},
    }
}
