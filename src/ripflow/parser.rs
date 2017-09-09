//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
//!
//! Thanks to the scanner, the parser can actually read and parse a file of
//! theoretically infinite size incrementally (other than memory limits on
//! storing the resulting AST).
use ast::*;
use scanner::Scanner;


/// Describes an error that occured in parsing.
pub struct ParseError {
    pub name: String,
    pub line: u32,
    pub column: u32,
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
                panic!("expected }");
            }

            Ok(block)
        },
        Some('(') => self.parse_list(),
        Some(')') => parse_error!(self, ParseErrorKind::TrailingParenthesis),
        Some('"') | Some('\'') => self.parse_string(),
        Some(_) => self.parse_symbol(),
        None => {
            panic!("unexpected eof, expected expression");
        }
    }
}

fn parse_list(scanner: &mut Scanner) -> Result<Expression, ParseError> {
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

fn parse_symbol(scanner: &mut Scanner) -> Result<Expression, ParseError> {
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
