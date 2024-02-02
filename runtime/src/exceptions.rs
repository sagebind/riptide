use super::{
    scope::Scope,
    value::Value,
};
use std::{fmt, io};

/// An exception thrown at runtime.
///
/// An exception is a value (typically a string) that is _thrown_ used to indicate some sort of error detected during
/// runtime of a program.
#[derive(Clone)]
pub struct Exception {
    /// The exception message.
    pub(crate) message: Value,

    /// The cause of this exception, if any.
    pub(crate) cause: Option<Box<Exception>>,

    /// Backtrace captured at the time the exception was thrown.
    pub(crate) backtrace: Vec<gc::Gc<Scope>>,

    /// An unrecoverable exception is one generated by the runtime that can be
    /// caught, but is always re-thrown.
    pub(crate) unrecoverable: bool,
}

impl Exception {
    /// Create a new exception with a message.
    ///
    /// Usually the message is a string, but technically it could be any value type.
    pub fn new<M: Into<Value>>(message: M) -> Self {
        Self {
            message: message.into(),
            cause: None,
            backtrace: vec![],
            unrecoverable: false,
        }
    }

    /// Create a new exception with a message and another exception that caused this one.
    ///
    /// Causes can be chained together, almost like a linked list. This is useful for debugging, as it can provide a
    /// poor man's kind of "trace" of errors to help find the root cause.
    pub fn with_cause<M: Into<Value>>(message: M, cause: Exception) -> Self {
        Self {
            message: message.into(),
            cause: Some(Box::new(cause)),
            backtrace: vec![],
            unrecoverable: false,
        }
    }

    /// Create a new unrecoverable exception.
    pub(crate) fn unrecoverable(message: impl Into<Value>) -> Self {
        Self {
            message: message.into(),
            cause: None,
            backtrace: vec![],
            unrecoverable: true,
        }
    }

    /// Get the exception message.
    #[inline]
    pub fn message(&self) -> &Value {
        &self.message
    }

    /// Get the cause of the exception, if present.
    #[inline]
    pub fn cause(&self) -> Option<&Exception> {
        self.cause.as_ref().map(|x| x as _)
    }

    /// Check if this is an unrecoverable exception.
    #[inline]
    pub fn is_unrecoverable(&self) -> bool {
        self.unrecoverable
    }
}

impl From<Value> for Exception {
    fn from(message: Value) -> Self {
        Self::new(message)
    }
}

impl From<Exception> for Value {
    fn from(exception: Exception) -> Value {
        exception.message
    }
}

impl From<&'static str> for Exception {
    fn from(message: &'static str) -> Self {
        Self::new(message)
    }
}

impl From<String> for Exception {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}
impl From<io::Error> for Exception {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl fmt::Debug for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)?;

        for scope in &self.backtrace {
            write!(f, "\n  at {}", scope.name())?;
        }

        let mut cause = self.cause.as_ref();
        while let Some(c) = cause {
            write!(f, "\ncaused by: {}", c)?;
            cause = c.cause.as_ref();
        }

        Ok(())
    }
}
