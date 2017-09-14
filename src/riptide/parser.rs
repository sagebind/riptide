//! Parses script source code into an S-expression tree.
use expr::Expression;
use scanner::*;
use std::io::Read;
use std::rc::Rc;


/// A reference to a location in source code. Useful for error messages.
#[derive(Clone, Debug)]
pub struct SourceLocation {
    pub filename: Arc<String>,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    fn new<S: Into<String>>(filename: S) -> Self {
        Self {
            filename: Arc::new(filename.into()),
            line: 1,
            column: 1,
        }
    }
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self::new("<unknown>")
    }
}


pub fn parse_string(string: &str) -> Result<Expression, ParseError> {
    let mut scanner = StringScanner::new(string);
    let mut parser = Parser::new(&mut scanner);

    parser.parse_program()
}

pub fn parse_stream<R: Read>(reader: &mut R) -> Result<Expression, ParseError> {
    let mut scanner = ReaderScanner::new(reader);
    let mut parser = Parser::new(&mut scanner);

    parser.parse_program()
}


/// Parses a source input into an expression tree.
///
/// Since the grammar is so simple, this performs the role of both lexing and parsing.
pub struct Parser<'r> {
    scanner: &'r mut Scanner,
    next_char: Option<char>,
    pos: SourceLocation,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: SourceLocation,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    TrailingParenthesis,
    UnclosedList,
    UnclosedString,
}

impl ParseErrorKind {
    pub fn description(&self) -> &str {
        match self {
            &ParseErrorKind::TrailingParenthesis => "extra trailing parenthesis",
            &ParseErrorKind::UnclosedList => "unclosed list",
            &ParseErrorKind::UnclosedString => "unclosed string",
        }
    }
}

macro_rules! parse_error {
    ($parser:expr, $kind:expr) => (Err(ParseError {
        kind: $kind,
        pos: $parser.pos.clone(),
    }))
}

impl<'r> Parser<'r> {
    pub fn new(scanner: &'r mut Scanner) -> Self {
        Self {
            scanner: scanner,
            next_char: None,
            pos: SourceLocation::default(),
        }
    }

    /// Attempt to parse all input into an expression tree.
    pub fn parse_program(&mut self) -> Result<Expression, ParseError> {
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();

            if self.peek().is_some() {
                items.push(self.parse_expr()?);
            } else {
                break;
            }
        }

        Ok(Expression::List(items))
    }

    pub fn parse_expr(&mut self) -> Result<Expression, ParseError> {
        match self.peek() {
            Some('(') => self.parse_list(),
            Some(')') => parse_error!(self, ParseErrorKind::TrailingParenthesis),
            Some('"') | Some('\'') => self.parse_string(),
            Some(_) => self.parse_symbol(),
            None => {
                panic!("unexpected eof, expected expression");
            }
        }
    }

    fn parse_list(&mut self) -> Result<Expression, ParseError> {
        assert!(self.next() == Some('('));
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                Some(')') => {
                    self.next();
                    break;
                }
                Some(_) => items.push(self.parse_expr()?),
                None => return parse_error!(self, ParseErrorKind::UnclosedList)
            }
        }

        Ok(Expression::List(items))
    }

    fn parse_symbol(&mut self) -> Result<Expression, ParseError> {
        let mut string = String::new();

        // Read the first character of the symbol.
        string.push(self.next().expect("expected symbol"));

        // Read any remaining characters that are part of the symbol.
        while let Some(c) = self.peek() {
            match c {
                '(' | ')' | '"' | '\'' => break,
                c if c.is_whitespace() => break,
                c => {
                    self.next();
                    string.push(if c == '\\' {
                        self.next().map(translate_escape).unwrap_or('\\')
                    } else {
                        c
                    });
                },
            }
        }

        Ok(Expression::Atom(string.into()))
    }

    fn parse_string(&mut self) -> Result<Expression, ParseError> {
        let delimiter = match self.next() {
            Some(c) if c == '"' || c == '\'' => c,
            _ => panic!("invalid string delimiter"),
        };
        let mut string = String::new();

        loop {
            match self.next() {
                Some('\\') => string.push(self.next()
                    .map(translate_escape)
                    .unwrap_or('\\')),
                Some(c) if c == delimiter => break,
                Some(c) => string.push(c),
                None => return parse_error!(self, ParseErrorKind::UnclosedString),
            }
        }

        Ok(Expression::Atom(string.into()))
    }

    /// Consume and skip over any whitespace and comments.
    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(';') | Some('#') => {
                    loop {
                        match self.next() {
                            None | Some('\n') => break,
                            _ => continue,
                        }
                    }
                },
                Some(c) if c.is_whitespace() => {
                    self.next();
                },
                _ => break,
            }
        }
    }

    /// Get the next character of input.
    fn next(&mut self) -> Option<char> {
        match self.peek() {
            Some('\n') => {
                self.pos.line += 1;
                self.pos.column = 1;
            },
            Some(_) => {
                self.pos.column += 1;
            },
            None => {},
        }

        self.next_char.take()
    }

    /// Peek ahead one character.
    fn peek(&mut self) -> Option<char> {
        if self.next_char.is_none() {
            self.next_char = match self.scanner.read_char() {
                Ok(Some(c)) => Some(c),
                Ok(None) => None,
                Err(_) => None,
            };
        }

        self.next_char.clone()
    }
}

/// Get the value corresponding to a given escape character.
fn translate_escape(c: char) -> char {
    match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => c, // interpret all other chars as their literal
    }
}
