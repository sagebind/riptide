//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
//!
//! Thanks to the source, the parser can actually read and parse a file of
//! theoretically infinite size incrementally (other than memory limits on
//! storing the resulting AST).
use ast::Expr;
use reader::CharReader;
use std::borrow::Cow;


/// A reference to a location in source code. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,

    /// Offset fron the beginning of the source.
    pub offset: u32,
}

impl Default for SourcePos {
    fn default() -> SourcePos {
        SourcePos {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}


/// Describes an error that occured in parsing.
#[derive(Debug)]
pub struct ParseError {
    /// The error message. This is a string instead of an enum because the
    /// messages can be highly specific.
    pub message: Cow<'static, str>,

    /// The position in the source the error occurred in.
    pub pos: SourcePos,
}


pub struct Parser {
    reader: CharReader,
    next_char: Option<char>,
    pos: SourcePos,
}

impl Parser {
    /// Parse the characters from the given source into an AST.
    pub fn parse(mut self) -> Result<Expr, ParseError> {
        // Hey, a program is just a block!
        self.parse_block()
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {

            Some('{') => {
                let block = self.parse_block()?;

                if self.advance() == Some('}') {
                    return Err(self.error("expected }"));
                }

                Ok(block)
            },
            Some('(') => self.parse_function_call(),
            Some(')') => return Err(self.error("extra trailing parenthesis")),
            Some('"') | Some('\'') => self.parse_string(),
            Some(_) => self.parse_symbol(),
            None => {
                return Err(self.error("unexpected eof, expected expression"));
            }
        }
    }

    fn parse_block(&mut self) -> Result<Expr, ParseError> {
        // A block is just a series of 0 or more expressions.
        let mut statements = Vec::new();

        loop {
            let statement = self.parse_expr()?;
            statements.push(statement);

            if !self.match_any(&[';', '\n']) {
                break;
            }
        }

        Ok(Expr::Block(statements))
    }

    fn parse_function_call(&mut self) -> Result<Expr, ParseError> {
        assert!(self.advance() == Some('('));
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                Some(')') => {
                    self.advance();
                    break;
                },
                Some(_) => items.push(self.parse_expr()?),
                None => return Err(self.error("unclosed list")),
            }
        }

        Ok(Expr::Block(items))
    }

    fn parse_symbol(&mut self) -> Result<Expr, ParseError> {
        let mut string = String::new();

        // Read the first character of the symbol.
        string.push(self.advance().expect("expected symbol"));

        // Read any remaining characters that are part of the symbol.
        while let Some(c) = self.peek() {
            match c {
                '(' | ')' | '"' | '\'' => break,
                c if c.is_whitespace() => break,
                c => {
                    self.advance();
                    string.push(if c == '\\' {
                        self.advance().map(translate_escape).unwrap_or('\\')
                    } else {
                        c
                    });
                },
            }
        }

        Ok(Expr::ExpandableString(string.into()))
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        let delimiter = match self.advance() {
            Some(c) if c == '"' || c == '\'' => c,
            _ => panic!("invalid string delimiter"),
        };
        let mut string = String::new();

        loop {
            match self.advance() {
                Some('\\') => string.push(self.advance()
                    .map(translate_escape)
                    .unwrap_or('\\')),
                Some(c) if c == delimiter => break,
                Some(c) => string.push(c),
                None => return Err(self.error("ParseErrorKind::UnclosedString")),
            }
        }

        Ok(Expr::ExpandableString(string.into()))
    }

    /// Consume and skip over any whitespace and comments.
    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(';') | Some('#') => {
                    loop {
                        match self.advance() {
                            None | Some('\n') => break,
                            _ => continue,
                        }
                    }
                },
                Some(c) if c.is_whitespace() => {
                    self.advance();
                },
                _ => break,
            }
        }
    }

    fn peek(&mut self) -> Option<char> {
        if self.next_char.is_none() {
            self.next_char = match self.reader.read_char() {
                Ok(Some(c)) => Some(c),
                Ok(None) => None,
                Err(_) => None,
            };
        }

        self.next_char
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.next_char.take().or_else(|| {
            self.reader.read_char().unwrap()
        });

        self.pos.offset += 1;

        if c == Some('\n') {
            self.pos.line += 1;
            self.pos.column = 1;
        } else if c.is_some() {
            self.pos.column += 1;
        }

        c
    }

    fn match_any(&mut self, chars: &[char]) -> bool {
        for c in chars {
            if self.peek() == Some(*c) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn error<M: Into<Cow<'static, str>>>(&self, message: M) -> ParseError {
        ParseError {
            message: message.into(),
            pos: self.pos,
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
