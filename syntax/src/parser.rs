//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
use ast::*;
use super::ParseError;
use super::lexer::*;

pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        Self {
            lexer: lexer,
        }
    }

    pub fn parse_file(&mut self) -> Result<Block, ParseError> {
        let mut statements = Vec::new();

        loop {
            match self.lexer.peek()? {
                &Token::EndOfLine | &Token::EndOfStatement => {
                    self.lexer.advance()?;
                },
                &Token::EndOfFile => break,
                _ => {
                    statements.push(self.parse_pipeline()?);
                },
            }
        }

        Ok(Block {
            named_params: None,
            statements: statements,
        })
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
        let mut calls = Vec::new();
        calls.push(self.parse_function_call()?);

        while self.lexer.peek()? == &Token::Pipe {
            self.lexer.advance()?;
            calls.push(self.parse_function_call()?);
        }

        Ok(Pipeline {
            items: calls,
        })
    }

    fn parse_function_call(&mut self) -> Result<Call, ParseError> {
        let function = self.parse_expression()?;
        let mut args = Vec::new();

        loop {
            match self.lexer.peek()? {
                &Token::EndOfFile => break,
                &Token::EndOfLine => break,
                &Token::EndOfStatement => break,
                &Token::Pipe => break,
                &Token::RightBrace => break,
                &Token::RightParen => break,
                _ => args.push(self.parse_expression()?),
            }
        }

        Ok(Call {
            function: Box::new(function),
            args: args,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.lexer.peek()? {
            &Token::LeftBrace => self.parse_block_expr(),
            &Token::LeftBracket => self.parse_block_expr(),
            &Token::LeftParen => self.parse_pipeline_expr(),
            &Token::SubstitutionSigil => self.parse_substitution_expr(),
            &Token::Number(number) => {
                self.lexer.advance()?;
                Ok(Expr::Number(number))
            },
            _ => self.parse_string(),
        }
    }

    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        let named_params = match self.lexer.peek()? {
            &Token::LeftBracket => Some(self.parse_block_params()?),
            _ => None,
        };

        let statements = self.parse_block_body()?;

        Ok(Expr::Block(Block {
            named_params: named_params,
            statements: statements,
        }))
    }

    fn parse_block_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(Token::LeftBracket)?;
        let mut params = Vec::new();

        loop {
            match self.lexer.advance()? {
                Token::RightBracket => break,
                Token::String(s) => params.push(s),
                token => return Err(self.error(format!("unexpected token: {:?}", token))),
            }
        }

        Ok(params)
    }

    fn parse_block_body(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();

        loop {
            match self.lexer.peek()? {
                &Token::RightBrace => {
                    self.lexer.advance()?;
                    break;
                },
                &Token::EndOfFile => return Err(self.error("unterminated block")),
                _ => statements.push(self.parse_pipeline()?),
            }
        }

        Ok(statements)
    }

    fn parse_pipeline_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LeftParen)?;
        let pipeline = self.parse_pipeline()?;
        self.expect(Token::RightParen)?;

        Ok(Expr::Pipeline(pipeline))
    }

    fn parse_substitution_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::SubstitutionSigil)?;

        match self.lexer.advance()? {
            Token::String(s) => Ok(Expr::Substitution(Substitution {
                path: vec![s],
            })),
            token => Err(self.error(format!("expected string, instead got {:?}", token))),
        }
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        match self.lexer.advance()? {
            Token::String(s) => Ok(Expr::String(s)),
            Token::DoubleQuotedString(s) => Ok(Expr::ExpandableString(s)),
            token => Err(self.error(format!("expected string, instead got {:?}", token))),
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        let actual = self.lexer.advance()?;

        if actual == token {
            Ok(())
        } else {
            Err(self.error(format!("expected token: {:?}, instead got {:?}", token, actual)))
        }
    }

    fn advance_required(&mut self) -> Result<Token, ParseError> {
        match self.lexer.advance()? {
            Token::EndOfFile => Err(self.error("unexpected eof")),
            token => Ok(token),
        }
    }

    fn error<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError::new(message, self.lexer.file().pos())
    }
}
