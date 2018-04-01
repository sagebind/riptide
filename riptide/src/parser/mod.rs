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

mod lexer;
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

    pub fn parse(mut self) -> Result<Block, ParseError> {
        Ok(Block {
            named_params: None,
            statements: self.parse_statement_list()?,
        })
    }

    fn parse_statement_list(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        let mut statements = Vec::new();

        // loop {
        //     self.skip_whitespace();
        // }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Pipeline, ParseError> {
        self.parse_pipeline()
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
        unimplemented!();
    }

    fn parse_function_call(&mut self) -> Result<Call, ParseError> {
        let function = self.parse_expression()?;
        let mut args = Vec::new();

        // loop {}

        Ok(Call {
            function: Box::new(function),
            args: args,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.scanner.peek() {
            Some(b'{') => self.parse_block_expr(),
            Some(b'[') => self.parse_block_expr(),
            Some(b'(') => self.parse_pipeline_expr(),
            _ => self.parse_string(),
        }
    }

    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        let named_params = match self.scanner.peek() {
            Some(b'[') => Some(self.parse_block_params()?),
            _ => None,
        };

        let statements = self.parse_block_body()?;

        Ok(Expr::Block(Block {
            named_params: named_params,
            statements: statements,
        }))
    }

    fn parse_block_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(b'[')?;
        let mut params = Vec::new();

        loop {
            self.skip_horizontal_whitespace();

            match self.scanner.peek() {
                // End parameter list
                Some(b']') => {
                    self.scanner.advance();
                    break;
                },

                // Another param
                _ => params.push(self.parse_bare_string()?),
            }
        }

        Ok(params)
    }

    fn parse_block_body(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        self.expect(b'{')?;
        let statements = self.parse_statement_list()?;
        self.expect(b'}')?;

        Ok(statements)
    }

    fn parse_pipeline_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(b'(')?;
        let pipeline = self.parse_pipeline()?;
        self.expect(b')')?;

        Ok(Expr::Call(call))
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        match self.scanner.peek() {
            Some(b'\'') => self.parse_single_quoted_string(),
            Some(b'"') => self.parse_double_quoted_string(),
            _ => Ok(Expr::String(self.parse_bare_string()?)),
        }
    }

    fn parse_bare_string(&mut self) -> Result<String, ParseError> {
        let mut bytes = Vec::new();

        while let Some(c) = self.scanner.peek() {
            if is_unquoted_string_char(c) {
                bytes.push(c);
            } else {
                break;
            }
        }

        if bytes.is_empty() {
            return Err(self.error("expected bare string"));
        }

        match String::from_utf8(bytes) {
            Ok(string) => Ok(string),
            Err(_) => Err(self.error("invalid utf8")),
        }
    }

    fn parse_single_quoted_string(&mut self) -> Result<Expr, ParseError> {
        self.expect(b'\'')?;
        let mut bytes = Vec::new();

        loop {
            match self.advance_required()? {
                // End of the string
                b'\'' => break,

                // Character escape
                b'\\' => bytes.push(translate_escape(self.advance_required()?)),

                // Normal character
                c => bytes.push(c),
            }
        }

        match String::from_utf8(bytes) {
            Ok(string) => Ok(Expr::String(string)),
            Err(_) => Err(self.error("invalid utf8")),
        }
    }

    fn parse_double_quoted_string(&mut self) -> Result<Expr, ParseError> {
        self.expect(b'"')?;
        let mut bytes = Vec::new();

        loop {
            match self.advance_required()? {
                // End of the string
                b'"' => break,

                // Character escape
                b'\\' => bytes.push(translate_escape(self.advance_required()?)),

                // Normal character
                c => bytes.push(c),
            }
        }

        match String::from_utf8(bytes) {
            Ok(string) => Ok(Expr::String(string)),
            Err(_) => Err(self.error("invalid utf8")),
        }
    }

    fn skip_horizontal_whitespace(&mut self) {
        while let Some(c) = self.scanner.peek() {
            if is_horizontal_whitespace(c) {
                self.scanner.advance();
            } else {
                break;
            }
        }
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

    fn advance_required(&mut self) -> Result<u8, ParseError> {
        match self.scanner.advance() {
            Some(byte) => Ok(byte),
            None => Err(self.error("unexpected eof")),
        }
    }

    fn error<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError {
            message: message.into(),
            pos: self.scanner.pos().clone(),
        }
    }
}

fn is_unquoted_string_char(c: u8) -> bool {
    match c {
        b'_' | b'-' => true,
        c => c.is_ascii_alphanumeric(),
    }
}

fn is_horizontal_whitespace(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

/// Get the value corresponding to a given escape character.
fn translate_escape(c: u8) -> u8 {
    match c {
        b'\\' => b'\\',
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
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
            named_params: None,
            statements: vec![
                Expr::Call(Call {
                    function: Box::new(Expr::String("hello world".into())),
                    args: vec![],
                })
            ],
        }));
    }
}
