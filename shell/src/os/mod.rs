//! Asynchronous raw terminal primitives.

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::*;

// Windows 10 support might just require using this:
// https://docs.microsoft.com/en-us/windows/console/setconsolemode
