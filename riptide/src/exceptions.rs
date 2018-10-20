use std::fmt;
use std::io;
use value::Value;

/// An exception thrown at runtime.
///
/// An exception is a value (typically a string) that is _thrown_ used to indicate some sort of error detected during
/// runtime of a program.
#[derive(Clone)]
pub struct Exception {
    message: Value,
    cause: Option<Box<Exception>>,
}

impl Exception {
    /// Create a new exception with a message.
    ///
    /// Usually the message is a string, but technically it could be any value type.
    pub fn new<M: Into<Value>>(message: M) -> Self {
        Self {
            message: message.into(),
            cause: None,
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
        match self.cause {
            Some(ref cause) => Some(cause),
            None => None,
        }
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

        let mut cause = self.cause.as_ref();
        while let Some(c) = cause {
            write!(f, "caused by: {}", c.message)?;
            cause = c.cause.as_ref();
        }

        Ok(())
    }
}
