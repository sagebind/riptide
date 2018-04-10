/// Splits a source file into a stream of tokens.
use filemap::*;
use super::ParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    EndOfStatement,
    Pipe,
    Deref,
    Number(f64),
    DoubleQuotedString(String),
    String(String),
    EndOfLine,
    EndOfFile,
}

pub struct Lexer {
    file: FileMap,
    peeked: Option<Token>,
}

impl Lexer {
    /// Create a new lexer for the given file.
    pub fn new(file: FileMap) -> Lexer {
        Lexer {
            file: file,
            peeked: None,
        }
    }

    /// Get the file being lexed.
    pub fn file(&self) -> &FileMap {
        &self.file
    }

    /// Peek at the next token, if any, in the source.
    pub fn peek(&mut self) -> Result<&Token, ParseError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lex()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    /// Advance to the next token in the source.
    pub fn advance(&mut self) -> Result<Token, ParseError> {
        match self.peeked.take() {
            Some(token) => Ok(token),
            None => self.lex(),
        }
    }

    fn lex(&mut self) -> Result<Token, ParseError> {
        loop {
            match self.file.advance() {
                // Simple one-character tokens.
                Some(b'(') => return Ok(Token::LeftParen),
                Some(b')') => return Ok(Token::RightParen),
                Some(b'{') => return Ok(Token::LeftBrace),
                Some(b'}') => return Ok(Token::RightBrace),
                Some(b'[') => return Ok(Token::LeftBracket),
                Some(b']') => return Ok(Token::RightBracket),
                Some(b'|') => return Ok(Token::Pipe),
                Some(b'$') => return Ok(Token::Deref),
                Some(b';') => return Ok(Token::EndOfStatement),

                // Ignore horizontal whitespace.
                Some(b' ') | Some(0x09) | Some(0x0c) => continue,

                // Start of a line comment, ignore all following characters until end of line.
                Some(b'#') => {
                    loop {
                        match self.file.peek() {
                            Some(b'\r') | Some(b'\n') => break,
                            _ => self.file.advance(),
                        };
                    }
                    continue;
                },

                // To handle newlines in a platform-generic way, any of the following sequences are treated as a single
                // newline token: \r \r\n \n
                Some(b'\n') => return Ok(Token::EndOfLine),
                Some(b'\r') => {
                    if self.file.peek() == Some(b'\n') {
                        self.file.advance();
                    }
                    return Ok(Token::EndOfLine);
                },

                // Single-quoted string.
                Some(b'\'') => {
                    let mut bytes = Vec::new();

                    loop {
                        match self.file.advance() {
                            // End of the string.
                            Some(b'\'') => break,

                            // The only character escapes recognized in a single qouted string is \' and \\, so for all
                            // other characters we just proceed as normal.
                            Some(b'\\') => match self.file.peek() {
                                Some(b'\'') | Some(b'\\') => {
                                    bytes.push(self.file.advance().unwrap());
                                },
                                _ => bytes.push(b'\\'),
                            },

                            // Just a regular byte in the string.
                            Some(byte) => bytes.push(byte),

                            None => panic!("unexpected eof, expecting end of string '"),
                        }
                    }

                    return Ok(Token::String(String::from_utf8(bytes).unwrap()));
                },

                // Double quoted string.
                Some(b'"') => {
                    let mut bytes = Vec::new();

                    loop {
                        match self.file.advance() {
                            // End of the string
                            Some(b'"') => break,

                            // Character escape
                            Some(b'\\') => bytes.push(translate_escape(self.file.advance().unwrap())),

                            // Normal character
                            Some(byte) => bytes.push(byte),

                            None => panic!("unexpected eof, expecting end of string '"),
                        }
                    }

                    return Ok(Token::DoubleQuotedString(String::from_utf8(bytes).unwrap()));
                },

                // Number.
                Some(byte) if byte.is_ascii_digit() => {
                    let mut bytes = vec![byte];
                    let mut seen_decimal = false;

                    while let Some(byte) = self.file.peek() {
                        if byte == b'.' {
                            if seen_decimal {
                                panic!("unexpected '.'");
                            }
                            seen_decimal = true;
                            bytes.push(byte);
                            self.file.advance();
                        } else if byte.is_ascii_digit() {
                            bytes.push(byte);
                            self.file.advance();
                        } else {
                            break;
                        }
                    }

                    let string = unsafe {
                        String::from_utf8_unchecked(bytes)
                    };

                    return Ok(Token::Number(string.parse().unwrap()));
                },

                // Unquoted string.
                Some(byte) if is_unquoted_string_char(byte) => {
                    let mut bytes = vec![byte];

                    while let Some(byte) = self.file.peek() {
                        if !is_unquoted_string_char(byte) {
                            break;
                        }

                        self.file.advance();
                        bytes.push(byte);
                    }

                    return Ok(Token::String(String::from_utf8(bytes).unwrap()));
                },


                Some(_) => {
                    panic!("unexpected byte");
                },

                None => return Ok(Token::EndOfFile),
            }
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
