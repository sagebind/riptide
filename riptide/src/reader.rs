//! Provides types for reading files and strings incrementally.
use std::io::{self, Read};
use std::collections::VecDeque;
use utf8parse;


/// Reads characters from a stream incrementally.
pub struct CharReader {
    reader: Option<Box<Read>>,
    buffer: VecDeque<u8>,
    parser: utf8parse::Parser,
}

impl CharReader {
    const READ_SIZE: usize = 8192;

    /// Create a new source reader from a sequence of bytes.
    pub fn from_bytes<B: Into<Vec<u8>>>(bytes: B) -> Self {
        let bytes: Vec<u8> = bytes.into();
        Self {
            buffer: bytes.into(),
            reader: None,
            parser: utf8parse::Parser::new(),
        }
    }

    /// Create a new source reader from a byte reader.
    pub fn from_reader<R: Read + 'static>(reader: R) -> Self {
        Self {
            buffer: VecDeque::new(),
            reader: Some(Box::new(reader)),
            parser: utf8parse::Parser::new(),
        }
    }

    /// Read the next character.
    pub fn read_char(&mut self) -> io::Result<Option<char>> {
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

impl From<String> for CharReader {
    fn from(string: String) -> CharReader {
        CharReader::from_bytes(string)
    }
}

impl<'a> From<&'a str> for CharReader {
    fn from(string: &str) -> CharReader {
        CharReader::from_bytes(string)
    }
}

impl Iterator for CharReader {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        self.read_char().ok().and_then(|c| c)
    }
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;


    #[test]
    fn test_read_from_string() {
        let s = "hello world";
        let mut reader = CharReader::from_bytes(s);

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
        let mut reader = CharReader::from_reader(cursor);

        for expected_char in s.chars() {
            let actual_char = reader.next().unwrap();
            println!("{} == {}", actual_char, expected_char);
            assert!(actual_char == expected_char);
        }

        assert!(reader.next().is_none());
    }
}
