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
            match self.lexer.peek() {
                Some(&Token::LineTerminator) | Some(&Token::StatementTerminator) => {
                    self.lexer.advance();
                },
                Some(_) => {
                    statements.push(self.parse_pipeline()?);
                },
                None => break,
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

        while self.lexer.peek() == Some(&Token::Pipe) {
            self.lexer.advance();
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
            match self.lexer.peek() {
                Some(&Token::LineTerminator) => break,
                Some(&Token::Pipe) => break,
                Some(&Token::StatementTerminator) => break,
                None => break,
                _ => args.push(self.parse_expression()?),
            }
        }

        Ok(Call {
            function: Box::new(function),
            args: args,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.lexer.peek() {
            Some(&Token::LeftBrace) => self.parse_block_expr(),
            Some(&Token::LeftBracket) => self.parse_block_expr(),
            Some(&Token::LeftParen) => self.parse_pipeline_expr(),
            _ => self.parse_string(),
        }
    }

    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        let named_params = match self.lexer.peek() {
            Some(&Token::LeftBracket) => Some(self.parse_block_params()?),
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
            match self.lexer.advance() {
                Some(Token::RightBracket) => break,
                Some(Token::String(s)) => params.push(s),
                token => return Err(self.error(format!("unexpected token: {:?}", token))),
            }
        }

        Ok(params)
    }

    fn parse_block_body(&mut self) -> Result<Vec<Pipeline>, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();

        loop {
            match self.lexer.peek() {
                Some(&Token::RightBrace) => {
                    self.lexer.advance();
                    break;
                },
                Some(_) => statements.push(self.parse_pipeline()?),
                None => return Err(self.error("unterminated block")),
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

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        match self.lexer.advance() {
            Some(Token::String(s)) => Ok(Expr::String(s)),
            Some(Token::DoubleQuotedString(s)) => Ok(Expr::ExpandableString(s)),
            token => Err(self.error(format!("expected string, instead got {:?}", token))),
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        if self.lexer.advance().as_ref() == Some(&token) {
            Ok(())
        } else {
            Err(self.error(format!("expected token: {:?}", token)))
        }
    }

    fn advance_required(&mut self) -> Result<Token, ParseError> {
        match self.lexer.advance() {
            Some(token) => Ok(token),
            None => Err(self.error("unexpected eof")),
        }
    }

    fn error<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError {
            message: message.into(),
            pos: self.lexer.pos().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use filemap::FileMap;
    use super::*;

    #[test]
    fn parse_string() {
        let file = FileMap::buffer(None, "
            'hello world'
        ");
        let mut parser = Parser::new(Lexer::new(file));

        assert_eq!(parser.parse_file().unwrap(), Block {
            named_params: None,
            statements: vec![
                Pipeline {
                    items: vec![
                        Call {
                            function: Box::new(Expr::String("hello world".into())),
                            args: vec![],
                        }
                    ]
                }
            ],
        });
    }
}
