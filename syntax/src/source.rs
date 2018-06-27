//! Abstractions over reading files and source code used in the parser.
use std::borrow::Borrow;
use std::fs::File;
use std::io::{self, Read};
use std::ops::Index;
use std::path::Path;

/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,

    /// Byte offset from the beginning of the file.
    offset: usize,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

/// A span of characters in a source file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Span {
    /// The starting position.
    pub start: Position,

    /// The ending position.
    pub end: Position,
}

impl From<Position> for Span {
    fn from(pos: Position) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// Holds information about a source file being parsed in memory.
pub struct SourceFile {
    name: Option<String>,
    buffer: Vec<u8>,
}

impl SourceFile {
    /// Create a new file map using an in-memory buffer.
    pub fn buffer<N: Into<Option<String>>, B: Into<Vec<u8>>>(name: N, buffer: B) -> Self {
        Self {
            name: name.into(),
            buffer: buffer.into(),
        }
    }

    /// Create a new file map from a reader.
    pub fn file<N: Into<Option<String>>>(name: N, reader: &mut Read) -> io::Result<Self> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(Self::buffer(name, buffer))
    }

    /// Open a file as a file map.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let name = path.as_ref().file_name().map(|s| s.to_string_lossy().into_owned());
        let mut file = File::open(path)?;
        Self::file(name, &mut file)
    }

    /// Get the name of the file.
    pub fn name(&self) -> &str {
        self.name
            .as_ref()
            .map(String::as_str)
            .unwrap_or("<unknown>")
    }
}

impl Index<Position> for SourceFile {
    type Output = u8;

    fn index(&self, index: Position) -> &u8 {
        &self.buffer[index.offset]
    }
}

impl Index<Span> for SourceFile {
    type Output = [u8];

    fn index(&self, index: Span) -> &[u8] {
        &self.buffer[index.start.offset..index.end.offset]
    }
}

/// Iterates over a source file.
pub struct SourceCursor<F> {
    file: F,
    pos: Position,
    mark: Position,
}

impl<F: Borrow<SourceFile>> From<F> for SourceCursor<F> {
    fn from(file: F) -> Self {
        Self {
            file: file,
            pos: Position::default(),
            mark: Position::default(),
        }
    }
}

impl<F: Borrow<SourceFile>> SourceCursor<F> {
    /// Get the source file being iterated over.
    pub fn file(&self) -> &SourceFile {
        self.file.borrow()
    }

    /// Get the current position in the file.
    pub fn pos(&self) -> Position {
        self.pos
    }

    /// Get the current span in the file.
    pub fn span(&self) -> Span {
        Span {
            start: self.mark,
            end: self.pos,
        }
    }

    /// Peek at the next byte in the file.
    pub fn peek(&self) -> Option<u8> {
        self.file().buffer.get(self.pos.offset).cloned()
    }

    /// Set the mark to the current position.
    pub fn mark(&mut self) {
        self.mark = self.pos;
    }

    /// Advance to the next character.
    pub fn advance(&mut self) -> Option<u8> {
        let byte = self.file().buffer.get(self.pos.offset).cloned();
        match byte {
            Some(b'\n') => {
                self.pos.offset += 1;
                self.pos.line += 1;
                self.pos.column = 1;
                Some(b'\n')
            },
            Some(byte) => {
                self.pos.offset += 1;
                self.pos.column += 1;
                Some(byte)
            },
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_from_string() {
        let s = "hello world";
        let mut reader = SourceCursor::from(SourceFile::buffer(None, s));

        for expected in s.bytes() {
            assert_eq!(reader.advance(), Some(expected));
        }

        assert_eq!(reader.advance(), None);
    }
}
