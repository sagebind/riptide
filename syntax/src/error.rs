use source::*;
use std::fmt;

/// Describes an error that occurred in parsing.
pub struct ParseError {
    /// The error message. This is a string instead of an enum because the
    /// messages can be highly specific.
    pub message: String,

    /// The span in the source the error occurred in.
    pub span: Span,

    /// The source file the error occurred in.
    pub file: SourceFile,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span, file: SourceFile) -> Self {
        Self {
            message: message.into(),
            span,
            file,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}\n{}:{}:{}",
            self.message,
            self.file.name(),
            self.span.start.line,
            self.span.start.column,
        )?;

        let starting_offset = self.file
            .source()[..self.span.start.offset]
            .rfind("\n")
            .map(|offset| offset + 1)
            .unwrap_or(0);
        let ending_offset = self.file
            .source()[self.span.end.offset..]
            .find("\n")
            .map(|offset| offset + self.span.end.offset)
            .unwrap_or(self.file.len());

        for line in self.file.source()[starting_offset..ending_offset].lines() {
            writeln!(f, "| {}", line.trim())?;
        }

        Ok(())
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
