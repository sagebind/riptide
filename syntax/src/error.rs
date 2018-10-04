use parser::Rule;
use pest;
use source::*;
use std::fmt;

/// Describes an error that occurred in parsing.
pub struct ParseError {
    pub(crate) inner: pest::error::Error<Rule>,
    pub(crate) file: SourceFile,
}

impl ParseError {
    /// Get the source file the error occurred in.
    pub fn file(&self) -> &SourceFile {
        &self.file
    }

    /// Get the position in the file the error occurred.
    pub fn position(&self) -> (usize, usize) {
        match self.inner.location {
            pest::error::InputLocation::Pos(pos) => (pos, pos),
            pest::error::InputLocation::Span(span) => span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}
