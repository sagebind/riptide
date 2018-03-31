//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
//!
//! Thanks to the source, the parser can actually read and parse a file of
//! theoretically infinite size incrementally (other than memory limits on
//! storing the resulting AST).
use ast::*;
use std::io::Read;
use self::scanner::*;

mod scanner;

/// Describes an error that occured in parsing.
#[derive(Debug)]
pub struct ParseError {
    /// The error message. This is a string instead of an enum because the
    /// messages can be highly specific.
    pub message: String,

    /// The position in the source the error occurred in.
    pub pos: SourcePos,
}

pub struct Parser<R> {
    scanner: Scanner<R>,
}

impl<R: Read> Parser<R> {
    pub fn new<S: Into<Scanner<R>>>(source: S) -> Self {
        Self {
            scanner: source.into(),
        }
    }

    pub fn parse(mut self) -> Result<Expr, ParseError> {
        self.parse_block()
    }

    fn parse_block(&mut self) -> Result<Expr, ParseError> {
        unimplemented!();
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.scanner.peek() {
            Some(b'{') => {
                self.scanner.advance();
                let block = self.parse_block()?;
                self.expect(b'}')?;

                Ok(block)
            },
            Some(b'(') => {
                self.scanner.advance();
                let expr = self.parse_expression()?;
                self.expect(b')')?;

                Ok(expr)
            },
            _ => {
                unimplemented!();
            },
        }
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        unimplemented!();
    }

    fn parse_pipeline(&mut self) -> Result<Expr, ParseError> {
        unimplemented!();
    }

    /// Consume and skip over any whitespace and comments.
    fn skip_whitespace(&mut self) {
        loop {
            match self.scanner.peek() {
                Some(b'#') => {
                    loop {
                        match self.scanner.advance() {
                            None | Some(b'\n') => break,
                            _ => continue,
                        }
                    }
                },
                Some(c) if c.is_ascii_whitespace() => {
                    self.scanner.advance();
                },
                _ => break,
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        if self.scanner.advance() == Some(byte) {
            Ok(())
        } else {
            Err(self.error(format!("expected {}", byte as char)))
        }
    }

    fn error<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError {
            message: message.into(),
            pos: self.scanner.pos().clone(),
        }
    }
}

/// Get the value corresponding to a given escape character.
fn translate_escape(c: char) -> char {
    match c {
        '\\' => '\\',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => c, // interpret all other chars as their literal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_string() {
        let parser = Parser::new("
            'hello world'
        ");

        assert_eq!(parser.parse().unwrap(), Expr::Block(Block {
            statements: vec![],
        }));
    }
}
