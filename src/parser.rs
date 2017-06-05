//! Parses script source code into an S-expression tree.
use builtins;
use io::IO;
use scanner::*;
use std::borrow::Cow;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::sync::{Arc, Mutex};


#[derive(Clone, Debug)]
/// Abstract representation of an expression. An expression can either be an atom (string), or a list of expressions
/// surrounded by parenthesis.
pub enum Expression {
    /// A single value.
    Atom(Cow<'static, str>),

    /// An empty list.
    Nil,

    /// A list of expressions.
    List(Vec<Expression>),
}

impl Expression {
    pub const TRUE: Self = Expression::Atom(Cow::Borrowed("true"));
    pub const FALSE: Self = Expression::Atom(Cow::Borrowed("false"));

    /// Create a new atom.
    pub fn atom<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Expression::Atom(value.into())
    }

    /// Determine if this expression is equivalent to Nil, or ().
    pub fn is_nil(&self) -> bool {
        match self {
            &Expression::List(ref items) => items.is_empty(),
            &Expression::Atom(_) => false,
            &Expression::Nil => true,
        }
    }

    /// Determine if this expression is considered a truthy value.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Expression::List(ref items) => !items.is_empty(),
            &Expression::Atom(ref value) => !(value == "0" || value.is_empty() || value.to_lowercase() == "false"),
            &Expression::Nil => false,
        }
    }

    /// If this is an atom expression, get its value.
    pub fn value(&self) -> Option<&str> {
        if let &Expression::Atom(ref s) = self {
            Some(s)
        } else {
            None
        }
    }

    // If this is a non-empty list, return a reference to its contents.
    pub fn items(&self) -> Option<&[Expression]> {
        match self {
            &Expression::List(ref items) if items.len() > 0 => Some(items),
            _ => None,
        }
    }
}

impl cmp::PartialEq for Expression {
    fn eq(&self, rhs: &Self) -> bool {
        (self.is_nil() && rhs.is_nil()) || (self.value() == rhs.value())
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Expression::Atom(ref s) => write!(f, "{}", s),
            &Expression::Nil => write!(f, "()"),
            &Expression::List(ref v) => {
                write!(f, "(")?;
                let mut first = true;
                for expr in v {
                    if first {
                        write!(f, "{}", expr)?;
                        first = false;
                    } else {
                        write!(f, " {}", expr)?;
                    }
                }
                write!(f, ")")
            },
        }
    }
}


#[derive(Clone, Copy, Default, Debug)]
pub struct Pos {
    pub line: u32,
    pub column: u32,
}

impl Pos {
    fn new() -> Self {
        Self {
            line: 1,
            column: 1,
        }
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
    pos: Pos,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: Pos,
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
        pos: $parser.pos,
    }))
}

impl<'r> Parser<'r> {
    pub fn new(scanner: &'r mut Scanner) -> Self {
        Self {
            scanner: scanner,
            next_char: None,
            pos: Pos::new(),
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
                Some(';') => {
                    while self.next() != Some('\n') {}
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
