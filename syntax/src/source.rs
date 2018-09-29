//! Abstractions over reading files and source code used in the parser.

use pest;
use std::fs;
use std::io;
use std::path::Path;

/// A reference to a location in a source file. Useful for error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// The line number. Begins at 1.
    pub line: usize,

    /// The column position in the current line. Begins at 1.
    pub column: usize,

    /// Byte offset from the beginning of the file.
    pub offset: usize,
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

impl<'a> From<pest::Position<'a>> for Position {
    fn from(pos: pest::Position) -> Self {
        Self {
            line: pos.line_col().0,
            column: pos.line_col().1,
            offset: pos.pos(),
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

impl<'a> From<pest::Span<'a>> for Span {
    fn from(span: pest::Span) -> Self {
        Self {
            start: span.start_pos().into(),
            end: span.end_pos().into(),
        }
    }
}

/// Holds information about a source file being parsed in memory.
pub struct SourceFile {
    name: Option<String>,
    buffer: String,
}

impl SourceFile {
    /// Create a new file map using an in-memory buffer.
    pub fn buffer(name: impl Into<Option<String>>, buffer: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            buffer: buffer.into(),
        }
    }

    /// Open a file as a file map.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let name = path.file_name().map(|s| s.to_string_lossy().into_owned());

        fs::read_to_string(path).map(|string| Self::buffer(name, string))
    }

    /// Get the name of the file.
    pub fn name(&self) -> &str {
        self.name
            .as_ref()
            .map(String::as_str)
            .unwrap_or("<unknown>")
    }

    pub fn source(&self) -> &str {
        &self.buffer
    }
}
