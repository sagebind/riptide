use filemap::*;
use std::io;


#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Pipe,
    Deref,
    Whitespace,
    String(String),
    EndOfLine,
    EndOfFile,
}

impl TokenKind {
    fn string<S: Into<String>>(string: S) -> TokenKind {
        TokenKind::String(string.into())
    }
}


#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: FilePos,
}


pub struct Lexer {
    file: FileMap,
    next_byte: Option<u8>,
}

impl Lexer {
    /// Create a new lexer for the given file.
    pub fn new<F: Into<FileMap>>(file: F) -> Lexer {
        Lexer {
            file: file.into(),
            next_byte: None,
        }
    }

    /// Get the next token in the stream.
    pub fn next(&mut self) -> io::Result<Token> {
        let pos = self.file.pos();
        let kind = match self.next_byte()? {
            Some(byte) => match byte {
                // Simple one-character tokens.
                b'(' => TokenKind::LeftParen,
                b')' => TokenKind::RightParen,
                b'{' => TokenKind::LeftBrace,
                b'}' => TokenKind::RightBrace,
                b'|' => TokenKind::Pipe,
                b'$' => TokenKind::Deref,

                // Regular whitespace.
                b' ' | 0x09 | 0x0c => {
                    // Compress multiple whitespace characters into one token.
                    while let Some(byte) = self.peek_byte()? {
                        match byte {
                            b' ' | 0x09 | 0x0c => {self.next_byte()?;},
                            _ => break,
                        }
                    }

                    TokenKind::Whitespace
                },

                // To handle newlines in a platform-generic way, any of the following sequences are treated as a single
                // newline token: \r \r\n \n
                b'\n' => TokenKind::EndOfLine,
                b'\r' => {
                    if self.peek_byte()? == Some(b'\n') {
                        self.next_byte()?;
                    }
                    TokenKind::EndOfLine
                },

                // Single-quoted strings.
                b'\'' => {
                    let mut contents = Vec::new();

                    loop {
                        match self.next_byte()? {
                            // End of the string.
                            Some(b'\'') => break,

                            // The only character escapes recognized in a single qouted string is \' and \\, so for all
                            // other characters we just proceed as normal.
                            Some(b'\\') => match self.peek_byte()? {
                                Some(b'\'') | Some(b'\\') => {
                                    contents.push(self.next_byte()?.unwrap());
                                },
                                _ => contents.push(b'\\'),
                            },

                            // Just a regular byte in the string.
                            Some(byte) => contents.push(byte),

                            None => panic!("unexpected eof, expecting end of string '"),
                        }
                    }

                    TokenKind::String(String::from_utf8(contents).unwrap())
                }

                // Double quoted string.
                // b'"' => {}

                // Unquoted string.
                byte => {
                    let mut contents = Vec::new();
                    contents.push(byte);

                    while let Some(byte) = self.peek_byte()? {
                        if is_whitespace(byte) {
                            break;
                        }

                        self.next_byte()?;
                        contents.push(byte);
                    }

                    TokenKind::String(String::from_utf8(contents).unwrap())
                },
            },
            None => TokenKind::EndOfFile,
        };

        Ok(Token {
            kind: kind,
            pos: pos,
        })
    }

    fn peek_byte(&mut self) -> io::Result<Option<u8>> {
        if self.next_byte.is_none() {
            self.next_byte = self.file.next_byte()?;
        }

        Ok(self.next_byte.clone())
    }

    fn next_byte(&mut self) -> io::Result<Option<u8>> {
        if self.next_byte.is_none() {
            self.next_byte = self.file.next_byte()?;
        }

        Ok(self.next_byte.take())
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

fn is_horizontal_whitespace(byte: u8) -> bool {
    match byte {
        b' ' | 0x09 | 0x0c => true,
        _ => false,
    }
}

fn is_whitespace(byte: u8) -> bool {
    byte == 0x09 || byte == 0x0a || byte == 0x0c || byte == 0x0d || byte == 0x20
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_unquoted_strings() {
        assert_string_produces_tokens("echo foo bar", &[
            TokenKind::string("echo"),
            TokenKind::Whitespace,
            TokenKind::string("foo"),
            TokenKind::Whitespace,
            TokenKind::string("bar"),
            TokenKind::EndOfFile,
        ]);
    }

    #[test]
    fn test_single_quotes() {
        assert_string_produces_tokens("echo 'foo bar'", &[
            TokenKind::string("echo"),
            TokenKind::Whitespace,
            TokenKind::string("foo bar"),
            TokenKind::EndOfFile,
        ]);
    }

    fn assert_string_produces_tokens(string: &str, tokens: &[TokenKind]) {
        let mut lexer = Lexer::new(string);
        assert_tokens(&mut lexer, tokens);
    }

    fn assert_tokens(lexer: &mut Lexer, tokens: &[TokenKind]) {
        for kind in tokens {
            let token = lexer.next().unwrap();
            assert!(token.kind == *kind);
        }
    }
}
