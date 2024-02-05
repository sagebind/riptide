//! Helpers for navigating control flow within the interpreter.
//!
//! This is not exposed in the runtime API, as manipulating control flow is a
//! privileged operation.

use crate::{Exception, Value};

/// Control flow is handled in the interpreter entirely using return values.
///
/// When an expression is evaluated normally, it returns `Continue` with the
/// result of the expression. When normal control flow is interrupted with another
/// action, it returns `Break` with the action that took place, either an early
/// return or an exception thrown.
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

pub(crate) trait Resolve {
    fn resolve(self) -> Result<Value, Exception>;
}

impl Resolve for ControlFlow<Value> {
    fn resolve(self) -> Result<Value, Exception> {
        match self {
            ControlFlow::Continue(value) => Ok(value),
            ControlFlow::Break(BreakAction::Return(value)) => Ok(value),
            ControlFlow::Break(BreakAction::Throw(exception)) => Err(exception),
        }
    }
}

macro_rules! throw_cf {
    ($($arg:tt)*) => {
        return ::std::ops::ControlFlow::Break(BreakAction::Throw($crate::Exception::from(format!($($arg)*))))
    };
}

macro_rules! break_return {
    ($value:expr) => {
        return ::std::ops::ControlFlow::Break(BreakAction::Return($value))
    };
    () => {
        return ::std::ops::ControlFlow::Break(BreakAction::Return(Default::default()))
    };
}

pub(crate) use break_return;
pub(crate) use throw_cf;
