use std::io::{Bytes, Cursor, Read};

/// A reference to a location in source code. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,
}

impl Default for SourcePos {
    fn default() -> SourcePos {
        SourcePos {
            line: 1,
            column: 1,
        }
    }
}

pub struct Scanner<R> {
    reader: Bytes<R>,
    buffer: Option<u8>,
    pos: SourcePos,
}

impl<R: Read> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: reader.bytes(),
            buffer: None,
            pos: SourcePos::default(),
        }
    }

    pub fn pos(&self) -> &SourcePos {
        &self.pos
    }

    pub fn peek(&mut self) -> Option<u8> {
        if self.buffer.is_none() {
            self.buffer = self.reader.next().and_then(|r| r.ok());
        }

        self.buffer.clone()
    }

    pub fn advance(&mut self) -> Option<u8> {
        let byte = self.buffer.take().or_else(|| self.reader.next().and_then(|r| r.ok()));

        match byte {
            Some(b'\n') => {
                self.pos.line += 1;
                self.pos.column = 1;
            }
            Some(_) => {
                self.pos.column += 1;
            }
            _ => {},
        }

        byte
    }
}

impl<'a> From<&'a str> for Scanner<Cursor<&'a [u8]>> {
    fn from(string: &'a str) -> Scanner<Cursor<&'a [u8]>> {
        Scanner::from(string.as_bytes())
    }
}

impl<'a> From<&'a [u8]> for Scanner<Cursor<&'a [u8]>> {
    fn from(bytes: &'a [u8]) -> Scanner<Cursor<&'a [u8]>> {
        Scanner::new(Cursor::new(bytes))
    }
}
