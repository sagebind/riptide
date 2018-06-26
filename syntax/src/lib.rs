//! The core Riptide syntax implementation.
//!
//! The provided Riptide parser parses source code into a high-level abstract syntax tree, which can be used for
//! evaluation directly, optimization, formatting tools, etc.
use source::SourceFile;

pub mod ast;
pub mod errors;
pub mod lexer;
pub mod parser;
pub mod source;
pub mod tokens;

/// Attempts to parse a source file into an abstract syntax tree.
///
/// If the given file contains a valid Riptide program, a root AST node is returned representing the program. If the
/// program instead contains any syntax errors, the errors are returned instead.
pub fn parse(file: SourceFile) -> Result<ast::Block, errors::ParseError> {
    let lexer = lexer::Lexer::new(file);
    let mut parser = parser::Parser::new(lexer);

    parser.parse_file()
}
