//! Helpers for navigating control flow within the interpreter.
//!
//! This is not exposed in the runtime API, as manipulating control flow is a
//! privileged operation.

use crate::{Exception, Value};

/// Control flow is handled in the interpreter entirely using return values.
pub(crate) type ControlFlow<T> = std::ops::ControlFlow<BreakAction, T>;

/// When performing an early exit of normal control flow, this is the action being
/// performed.
pub(crate) enum BreakAction {
    /// Break out of the closest function boundary with the given return value.
    /// This bubbles up through the stack until the nearest function invocation
    /// is reached.
    Return(Value),

    /// Throw an exception. This is bubbled up through the stack until caught.
    Throw(Exception),
}

macro_rules! throw_cf {
    ($($arg:tt)*) => {
        return ::std::ops::ControlFlow::Break(BreakAction::Throw($crate::Exception::from(format!($($arg)*))))
    };
}

pub(crate) use throw_cf;
