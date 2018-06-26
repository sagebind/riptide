/// Splits a source file into a stream of tokens.
use source::*;
use std::borrow::Borrow;
use super::errors::ParseError;
use super::tokens::*;

/// Possible "modes" a lexer can be in.
///
/// Riptide cannot be tokenized context-free, so the lexer enters in and out of various modes that control tokenization
/// behavior.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LexerMode {
    /// Normal mode: This mode tokenizes regular source code. The lexer starts and ends in this mode.
    Normal,

    /// Interpolation mode: The lexer enters into this mode when it begins to tokenize a double-quoted string.
    Interpolation,
}

/// Tokenizes a file into a series of tokens.
pub struct Lexer<F> {
    cursor: SourceCursor<F>,
}

impl<F: Borrow<SourceFile>> From<F> for Lexer<F> {
    fn from(file: F) -> Self {
        Self {
            cursor: SourceCursor::from(file),
        }
    }
}

impl<F: Borrow<SourceFile>> Lexer<F> {
    /// Get the file being lexed.
    #[inline]
    pub fn file(&self) -> &SourceFile {
        self.cursor.file()
    }

    /// Advance to the next token in the source.
    ///
    /// The token will be lexed according to the rules for the given mode.
    pub fn lex(&mut self, mode: LexerMode) -> Result<TokenInfo, ParseError> {
        self.cursor.mark();

        match mode {
            LexerMode::Normal => Ok(self.create_token(self.lex_normal()?)),
            LexerMode::Interpolation => unimplemented!(),
        }
    }

    pub fn create_error<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError::new(message, self.cursor.pos())
    }

    fn lex_normal(&mut self) -> Result<Token, ParseError> {
        loop {
            match self.cursor.advance() {
                // Simple one-character tokens.
                Some(b'(') => return Ok(Token::LeftParen),
                Some(b')') => return Ok(Token::RightParen),
                Some(b'{') => return Ok(Token::LeftBrace),
                Some(b'}') => return Ok(Token::RightBrace),
                Some(b'[') => return Ok(Token::LeftBracket),
                Some(b']') => return Ok(Token::RightBracket),
                Some(b'|') => return Ok(Token::Pipe),
                Some(b':') => return Ok(Token::Colon),
                Some(b';') => return Ok(Token::EndOfStatement),

                // Could be the start of a simple substitution or a complex one.
                Some(b'$') => match self.cursor.peek() {
                    Some(b'(') => {
                        self.cursor.advance();
                        return Ok(Token::SubstitutionParen);
                    },
                    Some(b'{') => {
                        self.cursor.advance();
                        return Ok(Token::SubstitutionBrace);
                    },
                    _ => return Ok(Token::SubstitutionSigil),
                },

                // Ignore horizontal whitespace.
                Some(b' ') | Some(0x09) | Some(0x0c) => continue,

                // Start of a line comment, ignore all following characters until end of line.
                Some(b'#') => {
                    loop {
                        match self.cursor.peek() {
                            Some(b'\r') | Some(b'\n') => break,
                            _ => self.cursor.advance(),
                        };
                    }
                    continue;
                },

                // To handle newlines in a platform-generic way, any of the following sequences are treated as a single
                // newline token: \r \r\n \n
                Some(b'\n') => return Ok(Token::EndOfLine),
                Some(b'\r') => {
                    if self.cursor.peek() == Some(b'\n') {
                        self.cursor.advance();
                    }
                    return Ok(Token::EndOfLine);
                },

                // Single-quoted string.
                Some(b'\'') => return self.lex_single_quoted_string(),

                // Double quoted string.
                Some(b'"') => return self.lex_double_quoted_string(),

                // Number.
                Some(byte) if byte.is_ascii_digit() => return self.lex_number_literal(byte),

                // Unquoted string.
                Some(byte) if is_unquoted_string_char(byte) => return self.lex_unquoted_string(byte),

                Some(_) => return Err(self.create_error("unexpected byte")),

                None => return Ok(Token::EndOfFile),
            }
        }
    }

    fn lex_single_quoted_string(&mut self) -> Result<Token, ParseError> {
        let mut bytes = Vec::new();

        loop {
            match self.cursor.advance() {
                // End of the string.
                Some(b'\'') => break,

                // The only character escapes recognized in a single qouted string is \' and \\, so for all
                // other characters we just proceed as normal.
                Some(b'\\') => match self.cursor.peek() {
                    Some(b'\'') | Some(b'\\') => {
                        bytes.push(self.cursor.advance().unwrap());
                    },
                    _ => bytes.push(b'\\'),
                },

                // Just a regular byte in the string.
                Some(byte) => bytes.push(byte),

                None => return Err(self.create_error("unexpected eof, expecting end of string '")),
            }
        }

        return Ok(Token::StringLiteral(String::from_utf8(bytes).unwrap()));
    }

    fn lex_double_quoted_string(&mut self) -> Result<Token, ParseError> {
        let mut bytes = Vec::new();

        loop {
            match self.cursor.advance() {
                // End of the string
                Some(b'"') => break,

                // Character escape
                Some(b'\\') => bytes.push(translate_escape(self.cursor.advance().unwrap())),

                // Normal character
                Some(byte) => bytes.push(byte),

                None => return Err(self.create_error("unexpected eof, expecting end of string '")),
            }
        }

        return Ok(Token::DoubleQuotedString(String::from_utf8(bytes).unwrap()));
    }

    fn lex_unquoted_string(&mut self, first_byte: u8) -> Result<Token, ParseError> {
        let mut bytes = vec![first_byte];

        while let Some(byte) = self.cursor.peek() {
            if !is_unquoted_string_char(byte) {
                break;
            }

            self.cursor.advance();
            bytes.push(byte);
        }

        return Ok(Token::StringLiteral(String::from_utf8(bytes).unwrap()));
    }

    fn lex_number_literal(&mut self, first_byte: u8) -> Result<Token, ParseError> {
        let mut bytes = vec![first_byte];
        let mut seen_decimal = false;

        while let Some(byte) = self.cursor.peek() {
            if byte == b'.' {
                if seen_decimal {
                    return Err(self.create_error("unexpected '.'"));
                }
                seen_decimal = true;
                bytes.push(byte);
                self.cursor.advance();
            } else if byte.is_ascii_digit() {
                bytes.push(byte);
                self.cursor.advance();
            } else {
                break;
            }
        }

        let string = unsafe {
            String::from_utf8_unchecked(bytes)
        };

        return Ok(Token::Number(string.parse().unwrap()));
    }

    fn create_token(&self, token: Token) -> TokenInfo {
        TokenInfo {
            token: token,
            span: self.cursor.span(),
        }
    }
}

/// Get the value corresponding to a given escape character.
fn translate_escape(byte: u8) -> u8 {
    match byte {
        b'\\' => b'\\',
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        _ => byte, // interpret all other chars as their literal
    }
}

fn is_whitespace(byte: u8) -> bool {
    byte == 0x09 || byte == 0x0a || byte == 0x0c || byte == 0x0d || byte == 0x20
}

fn is_unquoted_string_char(byte: u8) -> bool {
    match byte {
        b'_' | b'-' => true,
        byte => byte.is_ascii_alphanumeric(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_tokens {
        (
            $($source:expr => [
                $($token:expr,)*
            ];)*
        ) => {
            $({
                use $crate::lexer::Token::*;
                let mut lexer = Lexer::new(FileMap::buffer(None, $source));
                $(
                    assert_eq!(lexer.advance().unwrap(), $token);
                )*
            })*
        };
    }

    #[test]
    fn test_unquoted_strings() {
        assert_tokens!{
            "echo foo bar" => [
                String("echo".into()),
                String("foo".into()),
                String("bar".into()),
            ];
        }
    }

    #[test]
    fn test_single_quotes() {
        assert_tokens! {
            "echo 'foo bar'" => [
                String("echo".into()),
                String("foo bar".into()),
            ];
        }
    }

    #[test]
    fn test_simple_script() {
        assert_tokens! {
            r#"
            #!/usr/bin/env riptide

            def main {
                println "Hello world!"
            }

            main

            "# => [
                EndOfLine,
                EndOfLine,
                EndOfLine,
                String("def".into()),
                String("main".into()),
                LeftBrace,
                EndOfLine,
                String("println".into()),
                DoubleQuotedString("Hello world!".into()),
                EndOfLine,
                RightBrace,
                EndOfLine,
                EndOfLine,
                String("main".into()),
                EndOfLine,
                EndOfLine,
            ];
        }
    }
}
