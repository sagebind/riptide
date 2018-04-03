/// Splits a source file into a stream of tokens.
use filemap::*;
use std::iter::Peekable;

/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,
}

impl Default for SourcePos {
    fn default() -> SourcePos {
        SourcePos { line: 1, column: 1 }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Pipe,
    Deref,
    DoubleQuotedString(String),
    String(String),
    EndOfLine,
    EndOfFile,
}

pub struct Lexer {
    file: Peekable<FileMap>,
    pos: SourcePos,
    peeked: Option<Token>,
}

impl Lexer {
    /// Create a new lexer for the given file.
    pub fn new(file: FileMap) -> Lexer {
        Lexer {
            file: file.peekable(),
            pos: SourcePos::default(),
            peeked: None,
        }
    }

    /// Get the current position in the source.
    pub fn pos(&self) -> &SourcePos {
        &self.pos
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        while let Some(byte) = self.file.next() {
            return Some(match byte {
                // Simple one-character tokens.
                b'(' => Token::LeftParen,
                b')' => Token::RightParen,
                b'{' => Token::LeftBrace,
                b'}' => Token::RightBrace,
                b'|' => Token::Pipe,
                b'$' => Token::Deref,

                // Ignore horizontal whitespace.
                b' ' | 0x09 | 0x0c => continue,

                // Start of a line comment, ignore all following characters until end of line.
                b'#' => {
                    loop {
                        match self.file.peek() {
                            Some(&b'\r') | Some(&b'\n') => break,
                            _ => self.file.next(),
                        };
                    }
                    continue;
                },

                // To handle newlines in a platform-generic way, any of the following sequences are treated as a single
                // newline token: \r \r\n \n
                b'\n' => Token::EndOfLine,
                b'\r' => {
                    if self.file.peek() == Some(&b'\n') {
                        self.file.next();
                    }
                    Token::EndOfLine
                },

                // Single-quoted string.
                b'\'' => {
                    let mut bytes = Vec::new();

                    loop {
                        match self.file.next() {
                            // End of the string.
                            Some(b'\'') => break,

                            // The only character escapes recognized in a single qouted string is \' and \\, so for all
                            // other characters we just proceed as normal.
                            Some(b'\\') => match self.file.peek().cloned() {
                                Some(b'\'') | Some(b'\\') => {
                                    bytes.push(self.file.next().unwrap());
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
                        match self.file.next() {
                            // End of the string
                            Some(b'"') => break,

                            // Character escape
                            Some(b'\\') => bytes.push(translate_escape(self.file.next().unwrap())),

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

                    while let Some(byte) = self.file.peek().cloned() {
                        if is_whitespace(byte) {
                            break;
                        }

                        self.file.next();
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
                    assert_eq!(lexer.next().unwrap(), $token);
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
