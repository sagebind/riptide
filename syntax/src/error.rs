use crate::{
    grammar::Rule,
    source::{SourceFile, Span},
};
use std::fmt;

/// Describes an error that occurred in parsing.
pub struct ParseError {
    /// The error message details.
    variant: Variant,

    /// Where the error occurred.
    span: Span,
}

enum Variant {
    Pest(Box<pest::error::Error<Rule>>),
    Message(String),
}

impl ParseError {
    pub(crate) fn new(span: Span, message: String) -> Self {
        Self {
            variant: Variant::Message(message),
            span,
        }
    }

    pub(crate) fn from_pest(span: Span, error: pest::error::Error<Rule>) -> Self {
        Self {
            variant: Variant::Pest(Box::new(error.with_path(span.source_file().name().as_ref()))),
            span,
        }
    }

    /// Get the source file the error occurred in.
    pub fn file(&self) -> &SourceFile {
        self.span.source_file()
    }

    // / Get the position in the file the error occurred.
    // pub fn position(&self) -> (usize, usize) {
    //     match self.inner.location {
    //         pest::error::InputLocation::Pos(pos) => (pos, pos),
    //         pest::error::InputLocation::Span(span) => span,
    //     }
    // }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.variant {
            Variant::Pest(e) => fmt::Display::fmt(e, f),
            Variant::Message(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.variant {
            Variant::Pest(e) => fmt::Debug::fmt(e, f),
            Variant::Message(e) => fmt::Debug::fmt(e, f),
        }
    }
}
