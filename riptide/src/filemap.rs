//! Abstractions over reading files and source code used in the parser.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub struct FileMap {
    name: Option<String>,
    buffer: Vec<u8>,
    offset: usize,
}

impl FileMap {
    /// Create a new file map using an in-memory buffer.
    pub fn buffer<N: Into<Option<String>>, B: Into<Vec<u8>>>(name: N, buffer: B) -> Self {
        Self {
            name: name.into(),
            buffer: buffer.into(),
            offset: 0,
        }
    }

    /// Create a new file map from a reader.
    pub fn file<N: Into<Option<String>>>(name: N, reader: &mut Read) -> io::Result<Self> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(Self::buffer(name, buffer))
    }

    /// Open a file as a file map.
    pub fn open(path: &Path) -> io::Result<Self> {
        let name = path.file_name().map(|s| s.to_string_lossy().into_owned());
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

    pub fn peek(&self) -> Option<u8> {
        self.buffer.get(self.offset).cloned()
    }

    pub fn advance(&mut self) -> Option<u8> {
        match self.buffer.get(self.offset) {
            Some(byte) => {
                self.offset += 1;
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
