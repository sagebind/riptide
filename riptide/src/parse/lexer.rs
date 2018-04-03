/// Splits a source file into a stream of tokens.
use filemap::*;
use super::SourcePos;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Pipe,
    Deref,
    DoubleQuotedString(String),
    String(String),
    EndOfLine,
}

pub struct Lexer {
    file: FileMap,
    pos: SourcePos,
    peeked: Option<Token>,
}

impl Lexer {
    /// Create a new lexer for the given file.
    pub fn new(file: FileMap) -> Lexer {
        Lexer {
            file: file,
            pos: SourcePos::default(),
            peeked: None,
        }
    }

    /// Get the current position in the source.
    pub fn pos(&self) -> &SourcePos {
        &self.pos
    }

    /// Peek at the next token, if any, in the source.
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = self.lex();
        }
        self.peeked.as_ref()
    }

    /// Advance to the next token in the source.
    pub fn advance(&mut self) -> Option<Token> {
        match self.peeked.take() {
            Some(token) => Some(token),
            None => self.lex(),
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        match self.file.advance() {
            Some(b'\n') => {
                self.pos.line += 1;
                self.pos.column = 1;
                Some(b'\n')
            },
            Some(byte) => {
                self.pos.column += 1;
                Some(byte)
            },
            None => None,
        }
    }

    fn lex(&mut self) -> Option<Token> {
        while let Some(byte) = self.next_byte() {
            return Some(match byte {
                // Simple one-character tokens.
                b'(' => Token::LeftParen,
                b')' => Token::RightParen,
                b'{' => Token::LeftBrace,
                b'}' => Token::RightBrace,
                b'[' => Token::LeftBracket,
                b']' => Token::RightBracket,
                b';' => Token::Semicolon,
                b'|' => Token::Pipe,
                b'$' => Token::Deref,

                // Ignore horizontal whitespace.
                b' ' | 0x09 | 0x0c => continue,

                // Start of a line comment, ignore all following characters until end of line.
                b'#' => {
                    loop {
                        match self.file.peek() {
                            Some(b'\r') | Some(b'\n') => break,
                            _ => self.next_byte(),
                        };
                    }
                    continue;
                },

                // To handle newlines in a platform-generic way, any of the following sequences are treated as a single
                // newline token: \r \r\n \n
                b'\n' => Token::EndOfLine,
                b'\r' => {
                    if self.file.peek() == Some(b'\n') {
                        self.next_byte();
                    }
                    Token::EndOfLine
                },

                // Single-quoted string.
                b'\'' => {
                    let mut bytes = Vec::new();

                    loop {
                        match self.next_byte() {
                            // End of the string.
                            Some(b'\'') => break,

                            // The only character escapes recognized in a single qouted string is \' and \\, so for all
                            // other characters we just proceed as normal.
                            Some(b'\\') => match self.file.peek() {
                                Some(b'\'') | Some(b'\\') => {
                                    bytes.push(self.next_byte().unwrap());
                                },
                                _ => bytes.push(b'\\'),
                            },

                            // Just a regular byte in the string.
                            Some(byte) => bytes.push(byte),

                            None => panic!("unexpected eof, expecting end of string '"),
                        }
                    }

                    Token::String(String::from_utf8(bytes).unwrap())
                },

                // Double quoted string.
                b'"' => {
                    let mut bytes = Vec::new();

                    loop {
                        match self.file.advance() {
                            // End of the string
                            Some(b'"') => break,

                            // Character escape
                            Some(b'\\') => bytes.push(translate_escape(self.next_byte().unwrap())),

                            // Normal character
                            Some(byte) => bytes.push(byte),

                            None => panic!("unexpected eof, expecting end of string '"),
                        }
                    }

                    Token::DoubleQuotedString(String::from_utf8(bytes).unwrap())
                },

                // Unquoted string.
                byte if is_unquoted_string_char(byte) => {
                    let mut bytes = Vec::new();
                    bytes.push(byte);

                    while let Some(byte) = self.file.peek() {
                        if is_whitespace(byte) {
                            break;
                        }

                        self.next_byte();
                        bytes.push(byte);
                    }

                    Token::String(String::from_utf8(bytes).unwrap())
                },

                _ => {
                    panic!("unexpected byte");
                },
            });
        }

        None
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
                use $crate::parse::lexer::Token::*;
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
