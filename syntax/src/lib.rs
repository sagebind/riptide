//! The core Riptide syntax implementation.
//!
//! The provided Riptide parser parses source code into a high-level abstract syntax tree, which can be used for
//! evaluation directly, optimization, formatting tools, etc.

extern crate pest;
#[macro_use]
extern crate pest_derive;

use parser::FromPair;
use pest::Parser;
use source::*;
use std::fmt;

pub mod ast;
pub mod source;
mod parser;

/// Describes an error that occurred in parsing.
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

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Attempt to parse a source file into an abstract syntax tree.
///
/// If the given file contains a valid Riptide program, a root AST node is returned representing the program. If the
/// program instead contains any syntax errors, the errors are returned instead.
pub fn parse(file: SourceFile) -> Result<ast::Block, ParseError> {
    parser::Grammar::parse(parser::Rule::program, file.source())
        .map(|mut pairs| pairs.next().unwrap())
        .map(ast::Block::from_pair)
        .map_err(translate_error)
}

fn translate_error(error: pest::Error<'_, parser::Rule>) -> ParseError {
    match error {
        pest::Error::ParsingError {
            positives,
            negatives,
            pos,
        } => ParseError {
            message: {
                match (positives.is_empty(), negatives.is_empty()) {
                    (false, false) => format!(
                        "unexpected {}, expected {}",
                        negatives.iter()
                            .map(|rule| format!("{:?}", rule))
                            .collect::<Vec<_>>()
                            .join(" or "),
                        positives.iter()
                            .map(|rule| format!("{:?}", rule))
                            .collect::<Vec<_>>()
                            .join(" or "),
                    ),
                    (false, true) => format!(
                        "expected {}",
                        positives.iter()
                            .map(|rule| format!("{:?}", rule))
                            .collect::<Vec<_>>()
                            .join(" or "),
                    ),
                    (true, false) => format!(
                        "unexpected {}",
                        negatives.iter()
                            .map(|rule| format!("{:?}", rule))
                            .collect::<Vec<_>>()
                            .join(" or "),
                    ),
                    (true, true) => "unknown error".into(),
                }
            },
            span: source::Position::from(pos).into(),
        },
        pest::Error::CustomErrorPos {
            message,
            pos,
        } => ParseError {
            message: message,
            span: source::Position::from(pos).into(),
        },
        pest::Error::CustomErrorSpan {
            message,
            span,
        } => ParseError {
            message: message,
            span: span.into(),
        },
    }
}
