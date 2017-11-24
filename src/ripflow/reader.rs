//! Provides types for reading files and strings incrementally.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::collections::VecDeque;
use utf8parse;


/// A reference to a location in source code. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,

    /// Offset fron the beginning of the source.
    pub offset: u32,
}

impl Default for SourcePos {
    fn default() -> SourcePos {
        SourcePos {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}


/// Reads characters from a source incrementally.
pub struct SourceReader {
    name: String,
    /// An optional stream to read additional bytes from.
    reader: Option<Box<Read>>,
    /// Internal byte buffer.
    buffer: VecDeque<u8>,
    next_char: Option<char>,
    pos: SourcePos,
    parser: utf8parse::Parser,
}

impl SourceReader {
    const READ_SIZE: usize = 8192;
    const DEFAULT_NAME: &'static str = "<unknown>";

    /// Create a new source reader from a sequence of bytes.
    pub fn from_bytes<S, B>(name: S, bytes: B) -> Self
        where S: Into<String>,
              B: Into<Vec<u8>>,
    {
        let bytes: Vec<u8> = bytes.into();
        Self {
            name: name.into(),
            buffer: bytes.into(),
            reader: None,
            next_char: None,
            pos: SourcePos::default(),
            parser: utf8parse::Parser::new(),
        }
    }

    /// Create a new source reader from a byte reader.
    pub fn from_reader<S, R>(name: S, reader: R) -> Self
        where S: Into<String>,
              R: Read + 'static,
    {
        Self {
            name: name.into(),
            buffer: VecDeque::new(),
            reader: Some(Box::new(reader)),
            next_char: None,
            pos: SourcePos::default(),
            parser: utf8parse::Parser::new(),
        }
    }

    /// Open a file for reading.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let name = path.to_string_lossy().into_owned();

        Ok(Self::from_reader(name, File::open(path)?))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Report the current source position.
    pub fn pos(&self) -> SourcePos {
        self.pos
    }

    /// Get the next character of input.
    pub fn next(&mut self) -> Option<char> {
        let c = self.next_char.take().or_else(|| {
            self.read_char().unwrap()
        });

        match c {
            Some('\n') => {
                self.pos.line += 1;
                self.pos.column = 1;
            },
            Some(_) => {
                self.pos.column += 1;
            },
            None => {},
        }

        c
    }

    /// Peek ahead one character.
    pub fn peek(&mut self) -> Option<char> {
        if self.next_char.is_none() {
            self.next_char = match self.read_char() {
                Ok(Some(c)) => Some(c),
                Ok(None) => None,
                Err(_) => None,
            };
        }

        self.next_char.clone()
    }

    fn read_char(&mut self) -> io::Result<Option<char>> {
        struct Receiver {
            last_char: Option<char>,
            invalid: bool,
        }

        impl utf8parse::Receiver for Receiver {
            fn codepoint(&mut self, c: char) {
                self.last_char = Some(c);
            }

            fn invalid_sequence(&mut self) {
                self.invalid = true;
            }
        }

        let mut receiver = Receiver {
            last_char: None,
            invalid: false,
        };

        loop {
            // If the buffer is empty, read some more from the input.
            if self.buffer.is_empty() {
                if let Some(ref mut reader) = self.reader {
                    let mut buf = [0; Self::READ_SIZE];
                    let read_size = reader.read(&mut buf)?;

                    // Reached EOF.
                    if read_size == 0 {
                        return Ok(None);
                    }

                    self.buffer.reserve(read_size);
                    self.buffer.extend(&buf[0..read_size]);
                }
            }

            if let Some(byte) = self.buffer.pop_front() {
                self.parser.advance(&mut receiver, byte);

                // Check for an invalid byte sequence.
                if receiver.invalid {
                    return Err(io::ErrorKind::InvalidData.into());
                }

                // If we decoded a character successfully, return it.
                if let Some(c) = receiver.last_char {
                    return Ok(Some(c));
                }
            } else {
                return Ok(None);
            }
        }
    }
}

impl From<String> for SourceReader {
    fn from(string: String) -> SourceReader {
        SourceReader::from_bytes(SourceReader::DEFAULT_NAME, string)
    }
}

impl<'a> From<&'a str> for SourceReader {
    fn from(string: &str) -> SourceReader {
        SourceReader::from_bytes(SourceReader::DEFAULT_NAME, string)
    }
}

impl Iterator for SourceReader {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        SourceReader::next(self)
    }
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;


    #[test]
    fn test_read_from_string() {
        let s = "hello world";
        let mut reader = SourceReader::from_bytes("<unknown>", s);

        for expected_char in s.chars() {
            let actual_char = reader.next().unwrap();
            println!("{} == {}", actual_char, expected_char);
            assert!(actual_char == expected_char);
        }

        assert!(reader.next().is_none());
    }

    #[test]
    fn test_read_from_reader() {
        let s = "hello world";
        let cursor = Cursor::new(s);
        let mut reader = SourceReader::from_reader("<unknown>", cursor);

        for expected_char in s.chars() {
            let actual_char = reader.next().unwrap();
            println!("{} == {}", actual_char, expected_char);
            assert!(actual_char == expected_char);
        }

        assert!(reader.next().is_none());
    }
}
