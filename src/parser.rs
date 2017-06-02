use scanner::Scanner;
use std::error::Error;
use std::fmt;


#[derive(Debug)]
/// Abstract representation of an expression. An expression can either be an atom (string), or a list of expressions
/// surrounded by parenthesis.
pub enum Expression {
    Atom(String),
    List(Vec<Expression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Expression::Atom(ref s) => write!(f, "'{}'", s),
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


/// Parses a source input into an expression tree.
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
    InvalidEscape,
    TrailingParenthesis,
    UnclosedList,
    UnclosedString,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        use self::ParseErrorKind::*;

        match self.kind {
            InvalidEscape => "invalid escape character",
            TrailingParenthesis => "extra trailing parenthesis",
            UnclosedList => "unclosed list",
            UnclosedString => "unclosed string",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error: {}\n    {}:{}", self.description(), self.pos.line, self.pos.column)
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
    pub fn parse(mut self) -> Result<Expression, ParseError> {
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

    // <expr> := <list> | <string> | <symbol>
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

    // <list> := ( <expr_list> )
    fn parse_list(&mut self) -> Result<Expression, ParseError> {
        assert!(self.next() != Some('('));
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

    // <symbol> := <identifier>
    fn parse_symbol(&mut self) -> Result<Expression, ParseError> {
        let mut string = String::new();

        // Read the first character of the symbol.
        string.push(self.next().expect("expected symbol"));

        // Read any remaining characters that are part of the symbol.
        while let Some(c) = self.peek() {
            if c == '(' || c == ')' || c == '"' || c.is_whitespace() {
                break;
            }

            self.next();
            string.push(c);
        }

        Ok(Expression::Atom(string))
    }

    fn parse_string(&mut self) -> Result<Expression, ParseError> {
        let delimiter = match self.next() {
            Some(c) if c == '"' || c == '\'' => c,
            _ => panic!("invalid string delimiter"),
        };
        let mut string = String::new();

        loop {
            match self.next() {
                Some('\\') => {
                    string.push(match self.peek() {
                        Some(c) if c == '"' || c == '\'' => c,
                        _ => '\\',
                    });
                },
                Some(c) if c == delimiter => {
                    break;
                },
                Some(c) => {
                    string.push(c);
                },
                None => return parse_error!(self, ParseErrorKind::UnclosedString),
            }
        }

        Ok(Expression::Atom(string))
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
