use std::fmt;
use std::io;
use value::Value;

#[derive(Clone, Debug)]
pub struct Exception {
    message: Value,
    cause: Option<Box<Exception>>,
}

impl Exception {
    pub fn new<M: Into<Value>>(message: M) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    pub fn with_cause<M: Into<Value>>(message: M, cause: Exception) -> Self {
        Self {
            message: message.into(),
            cause: Some(Box::new(cause)),
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
