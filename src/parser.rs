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


pub struct Parser<'r> {
    scanner: &'r mut Scanner,
    next_char: Option<char>,
    line: u32,
    col: u32,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
    pub col: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnclosedList,
    UnclosedString,
    TrailingParenthesis,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        use self::ParseErrorKind::*;

        match self.kind {
            UnclosedList => "unclosed list",
            UnclosedString => "unclosed string",
            TrailingParenthesis => "extra trailing parenthesis",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.description())
    }
}

impl<'r> Parser<'r> {
    pub fn new(scanner: &'r mut Scanner) -> Self {
        Self {
            scanner: scanner,
            next_char: None,
            line: 1,
            col: 1,
        }
    }

    // <script> := <expr_list>
    pub fn parse_script(mut self) -> Result<Expression, ParseError> {
        let items = self.parse_expr_list()?;

        Ok(Expression::List(items))
    }

    // <expr> := <list> | <string> | <symbol>
    pub fn parse_expr(&mut self) -> Result<Expression, ParseError> {
        match self.peek() {
            Some('(') => {
                return self.parse_list();
            }
            Some('"') | Some('\'') => {
                return self.parse_string();
            }
            Some(_) => {
                return self.parse_symbol();
            }
            None => {
                panic!("unexpected eof, expected expression");
            }
        }
    }

    // <list> := ( <expr_list> )
    fn parse_list(&mut self) -> Result<Expression, ParseError> {
        if self.next() != Some('(') {
            panic!("expected char: (");
        }

        let items = self.parse_expr_list()?;

        self.skip_whitespace();

        if self.next() != Some(')') {
            panic!("expected char: )");
        }

        Ok(Expression::List(items))
    }

    // <expr_list> := <expr> <expr_list> | EPSILON
    fn parse_expr_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut exprs = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                Some(')') | None => {
                    break;
                }
                Some(_) => {
                    exprs.push(self.parse_expr()?);
                }
            }
        }

        Ok(exprs)
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

    // <string> := " <literals> "
    fn parse_string(&mut self) -> Result<Expression, ParseError> {
        let delimiter = match self.next() {
            Some(c) if c == '"' || c == '\'' => c,
            _ => panic!("expected string"),
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
                None => return Err(self.error(ParseErrorKind::UnclosedString)),
            }
        }

        Ok(Expression::Atom(string))
    }

    fn skip_whitespace(&mut self) {
        loop {
            if let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.next();
                    continue;
                }
            }
            break;
        }
    }

    fn peek(&mut self) -> Option<char> {
        if self.next_char.is_none() {
            self.next_char = match self.scanner.read_char() {
                Ok(Some('\n')) => {
                    self.line += 1;
                    self.col = 1;
                    Some('\n')
                },
                Ok(Some(c)) => {
                    Some(c)
                },
                Ok(None) => {
                    None
                },
                Err(_) => {
                    None
                },
            };
        }

        self.next_char.clone()
    }

    fn next(&mut self) -> Option<char> {
        self.peek();
        self.next_char.take()
    }

    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            kind: kind,
            line: self.line,
            col: self.col,
        }
    }
}
