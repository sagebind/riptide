use std::fmt;
use super::source::SourcePos;

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

    /// The position in the source the error occurred in.
    pub pos: SourcePos,
}

impl ParseError {
    pub fn new<S: Into<String>>(message: S, pos: SourcePos) -> Self {
        Self {
            message: message.into(),
            pos: pos,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.pos.line, self.pos.column, self.message)
    }
}
