use std::io::{self, Read};
use std::str::Chars;
use utf8parse;


/// Reads characters incrementally.
pub trait Scanner {
    /// Read the next character.
    fn read_char(&mut self) -> io::Result<Option<char>>;
}


/// Reads characters from a string.
pub struct StringScanner<'s> {
    chars: Chars<'s>,
}

impl<'s> StringScanner<'s> {
    pub fn new(string: &'s str) -> Self {
        Self {
            chars: string.chars(),
        }
    }
}

impl<'s> Scanner for StringScanner<'s> {
    fn read_char(&mut self) -> io::Result<Option<char>> {
        Ok(self.chars.next())
    }
}


/// Reads characters from a byte reader.
pub struct ReaderScanner<'r> {
    input: &'r mut Read,
    buffer: [u8; 8192],
    len: usize,
    cursor: usize,
    parser: utf8parse::Parser,
}

impl<'r> ReaderScanner<'r> {
    pub fn new<R: Read>(reader: &'r mut R) -> Self {
        Self {
            input: reader,
            buffer: [0; 8192],
            len: 0,
            cursor: 0,
            parser: utf8parse::Parser::new(),
        }
    }
}

impl<'r> Scanner for ReaderScanner<'r> {
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
            if self.cursor >= self.len {
                match self.input.read(&mut self.buffer)? {
                    // End of the stream.
                    0 => return Ok(None),
                    // Bytes read, reset the buffer.
                    len => {
                        self.len = len;
                        self.cursor = 0;
                    },
                }
            }

            // Advance the parser with the next byte.
            self.parser.advance(&mut receiver, self.buffer[self.cursor]);

            // Check for an invalid byte sequence.
            if receiver.invalid {
                return Ok(None);
            }

            // If we decoded a character successfully, return it.
            if let Some(c) = receiver.last_char {
                self.cursor += c.len_utf8();
                return Ok(Some(c));
            }
        }
    }
}
