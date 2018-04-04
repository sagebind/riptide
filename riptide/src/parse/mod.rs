use ast::Block;
use filemap::FileMap;
use std::fmt;

mod lexer;
mod parser;

/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,
}

impl Default for SourcePos {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
        }
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

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.pos.line, self.pos.column, self.message)
    }
}

/// Attempts to parse a source file into an abstract syntax tree.
pub fn parse(file: FileMap) -> Result<Block, ParseError> {
    let lexer = lexer::Lexer::new(file);
    let mut parser = parser::Parser::new(lexer);

    parser.parse_file()
}
