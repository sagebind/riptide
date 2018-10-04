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

pub mod ast;
pub mod error;
pub mod source;
mod parser;

/// Attempt to parse a source file into an abstract syntax tree.
///
/// If the given file contains a valid Riptide program, a root AST node is returned representing the program. If the
/// program instead contains any syntax errors, the errors are returned instead.
pub fn parse(file: SourceFile) -> Result<ast::Block, error::ParseError> {
    parser::Grammar::parse(parser::Rule::program, file.source())
        .map(|mut pairs| pairs.next().unwrap())
        .map(ast::Block::from_pair)
        .map_err(|e| translate_error(e, file.clone()))
}

fn translate_error(error: pest::Error<'_, parser::Rule>, file: SourceFile) -> error::ParseError {
    match error {
        pest::Error::ParsingError {
            positives,
            negatives,
            pos,
        } => error::ParseError {
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
            file,
        },
        pest::Error::CustomErrorPos {
            message,
            pos,
        } => error::ParseError {
            message: message,
            span: source::Position::from(pos).into(),
            file,
        },
        pest::Error::CustomErrorSpan {
            message,
            span,
        } => error::ParseError {
            message: message,
            span: span.into(),
            file,
        },
    }
}
