use std::fmt;
use super::source::*;

#[derive(Debug)]
pub struct ErrorList {
    errors: Vec<ParseError>,
}

impl From<ParseError> for ErrorList {
    fn from(error: ParseError) -> ErrorList {
        ErrorList {
            errors: vec![error],
        }
    }
}

impl fmt::Display for ErrorList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for error in self.errors.iter() {
            writeln!(f, "{}", error)?;
        }

        Ok(())
    }
}

/// Describes an error that occured in parsing.
#[derive(Debug)]
pub struct ParseError {
    /// The error message. This is a string instead of an enum because the
    /// messages can be highly specific.
    pub message: String,

    /// The span in the source the error occurred in.
    pub span: Span,
}

impl ParseError {
    pub fn new<S: Into<String>>(message: S, span: Span) -> Self {
        Self {
            message: message.into(),
            span: span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.start.line, self.span.start.column, self.message)
    }
}
