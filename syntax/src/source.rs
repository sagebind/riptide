//! Abstractions over reading files and source code used in the parser.

use std::{
    fmt,
    fs,
    io,
    path::Path,
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span {
    pub(crate) file_name: Option<String>,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file_name.as_deref().unwrap_or("<unknown>"), self.start.line, self.start.col)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position {
    pub(crate) line: u32,
    pub(crate) col: u32,
}

/// Holds information about a source file being parsed in memory.
#[derive(Clone, Debug)]
pub struct SourceFile {
    name: Option<String>,
    buffer: Rc<String>,
}

impl SourceFile {
    /// Create a named file map using an in-memory buffer.
    pub fn named(name: impl Into<String>, buffer: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            buffer: Rc::new(buffer.into()),
        }
    }

    /// Open a file as a file map.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        fs::read_to_string(path).map(|string| {
            let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap();
            Self::named(name, string)
        })
    }

    /// Get the name of the file.
    pub fn name(&self) -> &str {
        self.name.as_ref().map(String::as_str).unwrap_or("<unknown>")
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn source(&self) -> &str {
        &self.buffer
    }
}

impl<'a> From<&'a str> for SourceFile {
    fn from(string: &str) -> Self {
        String::from(string).into()
    }
}

impl From<String> for SourceFile {
    fn from(string: String) -> Self {
        Self {
            name: None,
            buffer: Rc::new(string),
        }
    }
}
