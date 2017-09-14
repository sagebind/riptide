//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
//!
//! Thanks to the scanner, the parser can actually read and parse a file of
//! theoretically infinite size incrementally (other than memory limits on
//! storing the resulting AST).
use rr::Expr;
use scanner::*;
use std::borrow::Cow;


pub struct Parser {
    scanner: Scanner,
    errors: Vec<ParseError>,
}

impl Parser {
    /// Create a new parser around the given scanner.
    pub fn new(scanner: Scanner) -> Parser {

    }
}


/// The result of a parse operation.
#[derive(Debug)]
pub enum ParseResult {
    /// Parsing was successful and produced an expression.
    Ok(Expr),

    /// Parsing failed with a list of errors.
    Fail(Vec<ParseError>),
}

impl ParseResult {
    pub fn is_err(&self) -> bool {
        match self {
            &ParseResult::Ok(_) => false,
            _ => true,
        }
    }

    pub fn accumulate_error(mut self, error: ParseError) -> Self {
        match self {
            ParseResult::Ok(_) => ParseResult::Fail(vec![error]),
            ParseResult::Fail(mut errors) => {
                errors.push(error);
                ParseResult::Fail(errors)
            },
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

impl ParseError {
    pub fn new<M, P>(message: M, pos: SourcePos) -> ParseError
        where M: Into<Cow<'static, str>>
    {
        ParseError {
            message: message.into(),
            pos: pos,
        }
    }
}

/// Helper macro for creating parse errors.
macro_rules! parse_error {
    ($message:expr, $scanner:expr) => {
        return Err(ParseError::new($message, $scanner.pos().clone()))
    }
}



/// Parse the characters from the given scanner into an AST.
pub fn parse(scanner: &mut Scanner) -> Result<Node, ParseError> {
    /// Hey, a program is just a block!
    parse_block(scanner)
}

fn parse_block(scanner: &mut Scanner) -> Result<Node, ParseError> {
    // A block is just a series of 0 or more expressions.
    Node::Nil
}

fn parse_expr(scanner: &mut Scanner) -> Result<Expression, ParseError> {
    match scanner.next() {
        Some('{') => {
            let block = parse_block(scanner)?;

            if scanner.next() == Some('}') {
                parse_error!("expected }", scanner);
            }

            Ok(block)
        },
        Some('(') => parse_list(scanner),
        Some(')') => parse_error!("extra trailing parenthesis", scanner),
        Some('"') | Some('\'') => self.parse_string(),
        Some(_) => parse_symbol(scanner),
        None => {
            parse_error!("unexpected eof, expected expression", scanner);
        }
    }
}

fn parse_list(scanner: &mut Scanner) -> Result<Expression, ParseError> {
    assert!(scanner.next() == Some('('));
    let mut items = Vec::new();

    loop {
        skip_whitespace(scanner);

        match scanner.peek() {
            Some(')') => {
                scanner.next();
                break;
            },
            Some(_) => items.push(parse_expr(scanner)?),
            None => parse_error!("unclosed list", scanner),
        }
    }

    Ok(Expression::List(items))
}

fn parse_symbol(scanner: &mut Scanner) -> Result<Expression, ParseError> {
    let mut string = String::new();

    // Read the first character of the symbol.
    string.push(scanner.next().expect("expected symbol"));

    // Read any remaining characters that are part of the symbol.
    while let Some(c) = scanner.peek() {
        match c {
            '(' | ')' | '"' | '\'' => break,
            c if c.is_whitespace() => break,
            c => {
                scanner.next();
                string.push(if c == '\\' {
                    scanner.next().map(translate_escape).unwrap_or('\\')
                } else {
                    c
                });
            },
        }
    }

    Ok(Expression::Atom(string.into()))
}

fn parse_string(scanner: &mut Scanner) -> Result<Expression, ParseError> {
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
