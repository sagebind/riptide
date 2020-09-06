//! Abstractions over reading files and source code used in the parser.

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt, fs, io,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

/// Holds information about a source file being parsed in memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile(Rc<Inner>);

#[derive(Debug, Eq, PartialEq)]
struct Inner {
    /// The file path. May be virtual.
    path: PathBuf,

    /// True if this file is virtual and doesn't actually exist on the file
    /// system.
    r#virtual: bool,

    /// The raw contents of the file.
    buffer: String,

    /// A "map" of line numbers (zero-based) to byte ranges in the buffer. Used
    /// for fast lookup of source text by line number.
    line_offsets: Vec<Range<usize>>,
}

impl SourceFile {
    /// Open a file as a file map.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        fs::read_to_string(path).map(|buffer| Self::new(path, false, buffer))
    }

    /// Create a "virtual" file using a provided path and in-memory buffer.
    pub fn r#virtual(path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self::new(path, true, contents)
    }

    fn new(path: impl Into<PathBuf>, r#virtual: bool, contents: impl Into<String>) -> Self {
        let buffer = contents.into();
        let mut line_offsets: Vec<Range<usize>> = Vec::new();

        for offset in 0..=buffer.len() {
            if let Some(b'\n') | None = buffer.as_bytes().get(offset) {
                let start = line_offsets.last().map(|range| range.start).unwrap_or(0);

                line_offsets.push(start..(offset + 1));
            }
        }

        Self(Rc::new(Inner {
            path: path.into(),
            r#virtual,
            line_offsets,
            buffer,
        }))
    }

    /// Get the name of the file.
    pub fn name(&self) -> Cow<'_, str> {
        self.0.path.file_name().unwrap().to_string_lossy()
    }

    /// Get the file size in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.buffer.len()
    }

    /// Check if this file is virtual.
    #[inline]
    pub fn is_virtual(&self) -> bool {
        self.0.r#virtual
    }

    /// Get an iterator over the lines in the file as an iterator of spans.
    pub fn lines(&self) -> impl Iterator<Item = Span> + '_ {
        self.0
            .line_offsets
            .iter()
            .enumerate()
            .map(move |(line, range)| Span {
                file: self.clone(),
                start: Position { line, offset: 0 },
                end: Position {
                    line,
                    offset: range.end - range.start - 1,
                },
            })
    }

    pub fn source_text(&self) -> &str {
        &self.0.buffer
    }

    /// Get the entire file as a span.
    pub fn span(&self) -> Span {
        Span {
            file: self.clone(),
            start: Position { line: 0, offset: 0 },
            end: Position {
                line: self.0.line_offsets.len() - 1,
                offset: self.0.line_offsets.last().unwrap().end,
            },
        }
    }

    pub fn slice(&self, start: usize, end: usize) -> Option<Span> {
        self.span().slice(start, end)
    }

    fn get_position(&self, offset: usize) -> Option<Position> {
        self.0
            .line_offsets
            .binary_search_by(|range| {
                if offset < range.start {
                    Ordering::Greater
                } else if offset >= range.end {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .map(|i| Position {
                line: i,
                offset: offset - self.0.line_offsets[i].start,
            })
            .ok()
    }

    fn get_offset(&self, position: Position) -> Option<usize> {
        let range = self.0.line_offsets.get(position.line)?;
        let offset = range.start + position.offset;

        if offset < range.end {
            Some(offset)
        } else {
            None
        }
    }
}

impl<'a> From<&'a str> for SourceFile {
    fn from(string: &str) -> Self {
        String::from(string).into()
    }
}

impl From<String> for SourceFile {
    fn from(string: String) -> Self {
        Self::r#virtual("<unknown>", string)
    }
}

impl AsRef<str> for SourceFile {
    fn as_ref(&self) -> &str {
        self.source_text()
    }
}

impl AsRef<[u8]> for SourceFile {
    fn as_ref(&self) -> &[u8] {
        self.source_text().as_bytes()
    }
}

/// A byte position in a source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// Line number, zero-based.
    line: usize,

    /// Byte offset from the start of the line, zero-based.
    offset: usize,
}

impl Position {
    #[inline]
    pub fn line(&self) -> usize {
        self.line + 1
    }

    #[inline]
    pub fn column(&self) -> usize {
        self.offset + 1
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line(), self.column())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    /// The file this span belongs to.
    file: SourceFile,

    /// Starting position for this span, inclusive.
    start: Position,

    /// Ending position for this span, inclusive.
    end: Position,
}

impl Span {
    #[inline]
    pub fn start(&self) -> Position {
        self.start
    }

    #[inline]
    pub fn end(&self) -> Position {
        self.end
    }

    /// Get the full source file this span is from.
    #[inline]
    pub fn source_file(&self) -> &SourceFile {
        &self.file
    }

    /// Get the source text for this span.
    pub fn source_text(&self) -> &str {
        &self.file.0.buffer
            [self.file.get_offset(self.start).unwrap()..self.file.get_offset(self.end).unwrap()]
    }

    pub fn slice(&self, start: usize, end: usize) -> Option<Span> {
        Some(Span {
            start: self.file.get_position(start)?,
            end: self.file.get_position(end)?,
            file: self.file.clone(),
        })
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file.name(), self.start)
    }
}

impl AsRef<str> for Span {
    fn as_ref(&self) -> &str {
        self.source_text()
    }
}

impl AsRef<[u8]> for Span {
    fn as_ref(&self) -> &[u8] {
        self.source_text().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let file = SourceFile::from("");

        assert_eq!(file.len(), 0);
        assert_eq!(file.lines().count(), 1);
        assert_eq!(file.get_position(0).unwrap().line(), 1);
        assert_eq!(file.get_position(0).unwrap().column(), 1);
    }
}
