//! Abstractions over reading files and source code used in the parser.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,
}

impl Default for SourcePos {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
        }
    }
}

pub struct FileMap {
    name: Option<String>,
    buffer: Vec<u8>,
    offset: usize,
    pos: SourcePos,
}

impl FileMap {
    /// Create a new file map using an in-memory buffer.
    pub fn buffer<N: Into<Option<String>>, B: Into<Vec<u8>>>(name: N, buffer: B) -> Self {
        Self {
            name: name.into(),
            buffer: buffer.into(),
            offset: 0,
            pos: SourcePos::default(),
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

    /// Get the current position in the file.
    pub fn pos(&self) -> SourcePos {
        self.pos
    }

    pub fn peek(&self) -> Option<u8> {
        self.buffer.get(self.offset).cloned()
    }

    pub fn advance(&mut self) -> Option<u8> {
        match self.buffer.get(self.offset) {
            Some(&b'\n') => {
                self.offset += 1;
                self.pos.line += 1;
                self.pos.column = 1;
                Some(b'\n')
            },
            Some(byte) => {
                self.offset += 1;
                self.pos.column += 1;
                Some(*byte)
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
        let mut reader = FileMap::buffer(None, s);

        for expected in s.bytes() {
            assert_eq!(reader.advance(), Some(expected));
        }

        assert_eq!(reader.advance(), None);
    }
}
