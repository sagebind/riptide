//! The language parser.
//!
//! This is a handwritten, recursive descent parser. This is done both for speed
//! and simplicity, since the language syntax is relatively simple anyway.
use ast::*;
use source::*;
use std::borrow::Borrow;
use super::errors::ParseError;
use super::lexer::*;
use super::tokens::*;

pub struct Parser<F> {
    lexer: Lexer<F>,
    peeked_token: Option<TokenInfo>,
}

impl<F: Borrow<SourceFile>> Parser<F> {
    pub fn new(lexer: Lexer<F>) -> Self {
        Self {
            lexer: lexer,
            peeked_token: None,
        }
    }

    pub fn parse_file(&mut self) -> Result<Block, ParseError> {
        let mut statements = Vec::new();

        loop {
            match self.peek()? {
                Token::EndOfLine | Token::EndOfStatement => {
                    self.advance()?;
                },
                Token::EndOfFile => break,
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

        while self.peek()? == Token::Pipe {
            self.advance()?;
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
            match self.peek()? {
                Token::EndOfFile => break,
                Token::EndOfLine => break,
                Token::EndOfStatement => break,
                Token::Pipe => break,
                Token::RightBrace => break,
                Token::RightParen => break,
                _ => args.push(self.parse_expression()?),
            }
        }

        Ok(Call {
            function: Box::new(function),
            args: args,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        debug!("parse expr, starting at {:?}", self.peek()?);

        match self.peek()? {
            Token::LeftBrace | Token::LeftBracket => self.parse_block_expr(),
            Token::LeftParen => self.parse_pipeline_expr(),
            Token::SubstitutionSigil => self.parse_substitution_expr(),
            Token::SubstitutionBrace => self.parse_substitution_expr(),
            Token::SubstitutionParen => self.parse_substitution_expr(),
            Token::NumberLiteral(number) => {
                self.advance()?;
                Ok(Expr::Number(number))
            },
            _ => self.parse_string(),
        }
    }

    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        let named_params = match self.peek()? {
            Token::LeftBracket => Some(self.parse_block_params()?),
            _ => None,
        };

        let statements = self.parse_block_body()?;

        Ok(Expr::Block(Block {
            named_params: named_params,
            statements: statements,
        }))
    }

    fn parse_block_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect_token(Token::LeftBracket)?;
        let mut params = Vec::new();

        loop {
            match self.advance()? {
                Token::RightBracket => break,
                Token::StringLiteral(s) => params.push(s.clone()),
                token => return Err(self.error(format!("unexpected token: {:?}", token))),
            }
        }

        Ok(params)
    }

    fn parse_block_body(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        self.expect_token(Token::LeftBrace)?;

        let mut statements = Vec::new();

        loop {
            match self.peek()? {
                Token::EndOfLine | Token::EndOfStatement => {
                    self.advance()?;
                },
                Token::RightBrace => {
                    self.advance()?;
                    break;
                },
                Token::EndOfFile => return Err(self.error("unterminated block")),
                _ => statements.push(self.parse_pipeline()?),
            }
        }

        Ok(statements)
    }

    fn parse_pipeline_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect_token(Token::LeftParen)?;
        let pipeline = self.parse_pipeline()?;
        self.expect_token(Token::RightParen)?;

        Ok(Expr::Pipeline(pipeline))
    }

    fn parse_substitution_expr(&mut self) -> Result<Expr, ParseError> {
        Ok(Expr::Substitution(self.parse_substitution()?))
    }

    fn parse_substitution(&mut self) -> Result<Substitution, ParseError> {
        match self.advance()? {
            Token::SubstitutionSigil => {
                Ok(Substitution::Variable(self.parse_variable_path()?))
            },

            Token::SubstitutionBrace => {
                let variable = self.parse_variable_path()?;

                match self.advance_required()? {
                    Token::Colon => {
                        let format_specifier = match self.advance_required()? {
                            Token::StringLiteral(string) => string.clone(),
                            Token::RightBrace => String::new(),
                            _ => return Err(self.error("expected format specifier")),
                        };

                        Ok(Substitution::Format(variable, Some(format_specifier)))
                    },
                    Token::RightBrace => Ok(Substitution::Format(variable, None)),
                    token => Err(self.error(format!("expected either ':' or '}}', instead got {:?}", token))),
                }
            },

            Token::SubstitutionParen => {
                let pipeline = self.parse_pipeline()?;
                self.expect_token(Token::RightParen)?;

                Ok(Substitution::Pipeline(pipeline))
            },

            _ => Err(self.error("expected substitution")),
        }
    }

    fn parse_variable_path(&mut self) -> Result<VariablePath, ParseError> {
        match self.advance()? {
            Token::StringLiteral(s) => Ok(VariablePath(vec![VariablePathPart::Ident(s)])),
            token => Err(self.error(format!("expected variable path, instead got {:?}", token))),
        }
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        match self.advance()? {
            Token::StringLiteral(s) => Ok(Expr::String(s)),
            Token::DoubleQuote => Ok(Expr::InterpolatedString(self.parse_interpolated_string()?)),
            token => Err(self.error(format!("expected string, instead got {:?}", token))),
        }
    }

    fn parse_interpolated_string(&mut self) -> Result<InterpolatedString, ParseError> {
        // Interpolated strings have their own lexer mode.
        self.lexer.push_mode(LexerMode::Interpolation);

        let mut parts = Vec::new();

        loop {
            // TODO
            match self.peek()? {
                Token::SubstitutionSigil
                    | Token::SubstitutionBrace
                    | Token::SubstitutionParen => {
                    self.lexer.push_mode(LexerMode::Normal);
                    parts.push(InterpolatedStringPart::Substitution(self.parse_substitution()?));
                    self.lexer.pop_mode();
                    continue;
                },
                _ => {},
            }

            match self.advance()? {
                // A region of regular text.
                Token::StringLiteral(s) => parts.push(InterpolatedStringPart::String(s)),

                // We've reached the end of the interpolated string.
                Token::DoubleQuote => break,

                _ => return Err(self.error("unexpected token")),
            }
        }

        // Restore the previous lexing mode.
        self.lexer.pop_mode();

        Ok(InterpolatedString(parts))
    }

    fn advance_required(&mut self) -> Result<Token, ParseError> {
        match self.advance()? {
            Token::EndOfFile => Err(self.error("unexpected eof")),
            token => Ok(token),
        }
    }

    /// Peek at the next token.
    fn peek(&mut self) -> Result<Token, ParseError> {
        if self.peeked_token.is_none() {
            self.peeked_token = Some(self.lexer.lex()?);
        }

        Ok(self.peeked_token.clone().unwrap().token)
    }

    /// Consume the current token, advancing to the next token in the file.
    fn advance(&mut self) -> Result<Token, ParseError> {
        match self.peeked_token.take() {
            Some(token) => Ok(token.token),
            None => Ok(self.lexer.lex()?.token),
        }
    }

    /// If the current token matches the given token, consume it, otherwise raise an error.
    fn expect_token(&mut self, token: Token) -> Result<(), ParseError> {
        let actual = self.advance()?;
        if actual == token {
            Ok(())
        } else {
            Err(self.error(format!("expected token: {:?}, instead got {:?}", token, actual)))
        }
    }

    fn error<S: Into<String>>(&self, message: S) -> ParseError {
        // TODO
        ParseError::new(message, Span::default())
    }
}
