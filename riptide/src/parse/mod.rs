use ast::Block;
use filemap::{FileMap, SourcePos};
use std::fmt;

mod lexer;
mod parser;

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

/// Attempts to parse a source file into an abstract syntax tree.
pub fn parse(file: FileMap) -> Result<Block, ParseError> {
    let lexer = lexer::Lexer::new(file);
    let mut parser = parser::Parser::new(lexer);

    parser.parse_file()
}
