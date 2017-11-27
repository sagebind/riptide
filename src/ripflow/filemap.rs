use std::fs::File;
use std::io::{self, BufReader, Bytes, Read};
use std::os::unix::io::*;
use std::path::Path;


/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FilePos {
    /// The line number. Begins at 1.
    pub line: u32,

    /// The column position in the current line. Begins at 1.
    pub column: u32,
}

impl Default for FilePos {
    fn default() -> FilePos {
        FilePos {
            line: 1,
            column: 1,
        }
    }
}


/// Line-aware source file reader that can be read incrementally as a stream of bytes.
pub struct FileMap {
    name: Option<String>,
    pos: FilePos,
    next_byte: Option<u8>,
    source: FileSource,
    // stream: Bytes<Box<Read>>,
}

enum FileSource {
    Buffer(Vec<u8>, usize),
    File(Bytes<BufReader<File>>),
}

impl FileMap {
    /// Create a new file map using an in-memory buffer.
    pub fn buffer<N: Into<Option<String>>, B: Into<Vec<u8>>>(name: N, buffer: B) -> Self {
        Self {
            name: name.into(),
            pos: FilePos::default(),
            next_byte: None,
            source: FileSource::Buffer(buffer.into(), 0),
        }
    }

    /// Create a new file map from an open file.
    pub fn file<N: Into<Option<String>>>(name: N, file: File) -> Self {
        Self {
            name: name.into(),
            pos: FilePos::default(),
            next_byte: None,
            source: FileSource::File(BufReader::new(file).bytes()),
        }
    }

    /// Open a file as a file map.
    pub fn open(path: &Path) -> io::Result<Self> {
        let name = path.file_name()
            .map(|s| s.to_string_lossy().into_owned());
        let file = File::open(path)?;

        Ok(Self::file(name, file))
    }

    /// Get the name of the file.
    pub fn name(&self) -> &str {
        self.name.as_ref().map(String::as_str).unwrap_or("<unknown>")
    }

    /// Get the current position in the file.
    pub fn pos(&self) -> FilePos {
        self.pos
    }

    /// Read the next byte from the underlying source.
    pub fn next_byte(&mut self) -> io::Result<Option<u8>> {
        match self.source {
            FileSource::Buffer(ref mut buf, ref mut pos) => {
                if *pos < buf.len() {
                    *pos += 1;
                    Ok(Some(buf[*pos - 1]))
                } else {
                    Ok(None)
                }
            },
            FileSource::File(ref mut r) => match r.next() {
                Some(Ok(b)) => Ok(Some(b)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            },
        }
    }
}

impl From<String> for FileMap {
    fn from(string: String) -> FileMap {
        FileMap::buffer(None, string)
    }
}

impl<'a> From<&'a str> for FileMap {
    fn from(string: &'a str) -> FileMap {
        FileMap::buffer(None, string)
    }
}

impl FromRawFd for FileMap {
    unsafe fn from_raw_fd(fd: RawFd) -> FileMap {
        let file = File::from_raw_fd(fd);
        Self::file(None, file)
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
            let actual = reader.next_byte().unwrap();
            assert!(actual == Some(expected));
        }

        assert!(reader.next_byte().unwrap().is_none());
    }
}

