//! The core Riptide syntax implementation.
//!
//! The provided Riptide parser parses source code into a high-level abstract syntax tree, which can be used for
//! evaluation directly, optimization, formatting tools, etc.

use crate::error::ParseError;
use crate::source::*;

pub mod ast;
pub mod error;
mod parser;
pub mod source;

/// Attempt to parse a source file into an abstract syntax tree.
///
/// If the given file contains a valid Riptide program, a root AST node is returned representing the program. If the
/// program instead contains any syntax errors, the errors are returned instead.
pub fn parse(file: impl Into<SourceFile>) -> Result<ast::Block, ParseError> {
    let file = file.into();

    parser::parse(file.clone())
        .map_err(|e| translate_error(e, file.clone()))
}

fn translate_error(error: pest::error::Error<parser::Rule>, file: SourceFile) -> ParseError {
    ParseError {
        inner: error.with_path(file.name()),
        file,
    }
}
