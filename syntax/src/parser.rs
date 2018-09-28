//! The language parser implementation.
//!
//! This is a handwritten, recursive descent parser that contains the bulk of the parsing code for the language syntax.
//! While there is not currently a formal specification of the language grammar, you can find a grammar definition in
//! the language reference document that is typically up-to-date with the implementation. You can also find a short
//! description of the rule being parsed by each routine in the doc comments.

use ast::*;
use source::*;
use std::borrow::Borrow;
use super::errors::ParseError;
use super::lexer::*;
use super::tokens::*;

/// A parser instance that manages parsing state.
pub struct Parser<F> {
    /// A lexer instance where tokens are parsed from.
    lexer: Lexer<F>,

    /// The current token being parsed.
    current_token: Option<TokenInfo>,
}

impl<F: Borrow<SourceFile>> Parser<F> {
    /// Create a new parser that parses tokens from the given lexer.
    pub fn new(lexer: Lexer<F>) -> Self {
        Self {
            lexer: lexer,
            current_token: None,
        }
    }

    /// Attempt to parse a source file into an abstract syntax tree.
    ///
    /// If the given file contains a valid Riptide program, a root AST node is returned representing the program. If the
    /// program instead contains any syntax errors, the errors are returned instead.
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

    /// Pipeline => FunctionCall (Pipe FunctionCall)*
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

    /// FunctionCall => Expr (Whitespace Expr)*
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

    /// Expr => BlockExpr
    ///       | PipelineExpr
    ///       | InterpolatedString
    ///       | NumberLiteral
    ///       | StringLiteral
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.peek()? {
            Token::LeftBrace
                | Token::LeftBracket => self.parse_block_expr(),
            Token::LeftParen => self.parse_pipeline_expr(),
            Token::SubstitutionSigil
                | Token::SubstitutionBrace
                | Token::SubstitutionParen => self.parse_substitution().map(Expr::Substitution),
            Token::DoubleQuote => self.parse_interpolated_string().map(Expr::InterpolatedString),
            Token::NumberLiteral(number) => {
                self.advance()?;
                Ok(Expr::Number(number))
            },
            Token::StringLiteral(s) => {
                self.advance()?;
                Ok(Expr::String(s))
            },
            token => Err(self.error(format!("expected expression, instead got {:?}", token))),
        }
    }

    /// BlockExpr => BlockParams? '{' BlockBody '}'
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

    /// BlockParams => '[' (Whitespace BareString)* Whitespace? ']'
    fn parse_block_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect([Token::LeftBracket])?;
        let mut params = Vec::new();

        loop {
            match self.advance()? {
                Token::RightBracket => break,
                Token::StringLiteral(s) => params.push(s),
                token => return Err(self.error(format!("unexpected token: {:?}", token))),
            }
        }

        Ok(params)
    }

    /// BlockBody           => '{' (Pipeline StatementSeparator)* Pipeline? '}'
    /// StatementSeparator  => LineTerminator+ | ';'
    fn parse_block_body(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        self.expect([Token::LeftBrace])?;

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

    /// PipelineExpr => '(' Pipeline ')'
    fn parse_pipeline_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect([Token::LeftParen])?;
        let pipeline = self.parse_pipeline()?;
        self.expect([Token::RightParen])?;

        Ok(Expr::Pipeline(pipeline))
    }

    fn parse_substitution(&mut self) -> Result<Substitution, ParseError> {
        match self.advance()? {
            Token::SubstitutionSigil => self.parse_variable_path().map(Substitution::Variable),

            Token::SubstitutionBrace => {
                let variable = self.parse_variable_path()?;

                match self.advance()? {
                    Token::Colon => {
                        let format_specifier = match self.peek()? {
                            Token::StringLiteral(string) => string,
                            _ => return Err(self.error("expected format specifier")),
                        };

                        self.advance()?;
                        self.expect([Token::RightBrace])?;

                        Ok(Substitution::Format(variable, Some(format_specifier)))
                    },

                    Token::RightBrace => Ok(Substitution::Format(variable, None)),

                    token => Err(self.error(format!("expected either ':' or '}}', instead got {:?}", token))),
                }
            },

            Token::SubstitutionParen => {
                let pipeline = self.parse_pipeline()?;
                self.expect([Token::RightParen])?;

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

    /// InterpolatedString      => '"' InterpolatedStringPart* '"'
    /// InterpolatedStringPart  => Substitution | StringLiteral
    fn parse_interpolated_string(&mut self) -> Result<InterpolatedString, ParseError> {
        // Interpolated strings have their own lexer mode.
        self.lexer.push_mode(LexerMode::Interpolation);

        self.expect([Token::DoubleQuote])?;

        let mut parts = Vec::new();

        loop {
            match self.peek()? {
                // Substitution embedded in the interpolated string.
                Token::SubstitutionSigil
                    | Token::SubstitutionBrace
                    | Token::SubstitutionParen => {
                    self.lexer.push_mode(LexerMode::Normal);
                    parts.push(InterpolatedStringPart::Substitution(self.parse_substitution()?));
                    self.lexer.pop_mode();
                },

                // A region of regular text.
                Token::StringLiteral(s) => {
                    self.advance()?;
                    parts.push(InterpolatedStringPart::String(s));
                },

                // We've reached the end of the interpolated string.
                Token::DoubleQuote => {
                    // Restore the previous lexing mode.
                    self.lexer.pop_mode();
                    self.advance()?;
                    break;
                },

                _ => return Err(self.error("unexpected token")),
            }
        }

        Ok(InterpolatedString(parts))
    }

    /// If the current token matches the given token, consume it, otherwise raise an error.
    fn expect(&mut self, tokens: impl AsRef<[Token]>) -> Result<(), ParseError> {
        for token in tokens.as_ref() {
            let current = self.peek()?;
            if &current != token {
                return Err(self.error(format!("expected token: {:?}, instead got {:?}", token, current)));
            }

            self.advance()?;
        }

        Ok(())
    }

    /// Get a reference to the current token being parsed, without consuming it.
    fn peek(&mut self) -> Result<Token, ParseError> {
        if self.current_token.is_none() {
            self.current_token = Some(self.lexer.lex()?);
        }
        Ok(self.current_token.clone().unwrap().token)
    }

    /// Consume the current token and return it, advancing to the next token in the file.
    fn advance(&mut self) -> Result<Token, ParseError> {
        let consumed = self.current_token.take().unwrap();
        self.current_token = Some(self.lexer.lex()?);
        Ok(consumed.token)
    }

    /// Construct a context-sensitive error message.
    fn error(&self, message: impl Into<String>) -> ParseError {
        self.current_token
            .as_ref()
            .map(|info| ParseError::new(message, info.span))
            .unwrap()
    }
}
