//! Supporting types for the parser to read characters from various sources
//! inrementally.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::Chars;
use utf8parse;


/// Reads source code from a source incrementally.
pub struct Scanner {
    pub source: Box<Source>,
    next_char: Option<char>,
    line: u32,
    column: u32,
}

impl Scanner {
    /// Create a scanner from a string.
    pub fn from_string<S: Into<String>>(string: S) -> Scanner {
        Scanner::new(StringSource::from(string.into()))
    }

    /// Open a file as a scanner.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Scanner> {
        FileSource::open(path).map(|source| Scanner::new(source))
    }

    /// Create a new scanner for a given source.
    pub fn new<S: Source + 'static>(source: S) -> Scanner {
        Scanner {
            source: Box::new(source),
            next_char: None,
            line: 1,
            column: 1,
        }
    }

    /// Report the current line number.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Report the current column number.
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Get the next character of input.
    pub fn next(&mut self) -> Option<char> {
        match self.peek() {
            Some('\n') => {
                self.line += 1;
                self.column = 1;
            },
            Some(_) => {
                self.column += 1;
            },
            None => {},
        }

        self.next_char.take()
    }

    /// Peek ahead one character.
    pub fn peek(&mut self) -> Option<char> {
        if self.next_char.is_none() {
            self.next_char = match self.source.read_char() {
                Ok(Some(c)) => Some(c),
                Ok(None) => None,
                Err(_) => None,
            };
        }

        self.next_char.clone()
    }
}


/// Represents a file or stream that can read incrementally.
pub trait Source {
    /// Get a file name being read from suitable for display.
    fn name(&self) -> &str {
        "<unknown>"
    }

    /// Read the next character.
    fn read_char(&mut self) -> io::Result<Option<char>>;
}


/// Reads characters from a string.
pub struct StringSource {
    string: String,
    index: usize,
}

impl From<String> for StringSource {
    fn from(string: String) -> StringSource {
        StringSource {
            string: string,
            index: 0,
        }
    }
}

impl Source for StringSource {
    fn read_char(&mut self) -> io::Result<Option<char>> {
        let c = unsafe {
            self.string.slice_unchecked(self.index, self.string.len()).chars().next()
        };

        Ok(match c {
            Some(c) => {
                self.index += c.len_utf8();
                Some(c)
            },
            None => None,
        })
    }
}


/// Reads characters from a file.
pub struct FileSource {
    name: String,
    file: File,
    buffer: [u8; 8192],
    len: usize,
    cursor: usize,
    parser: utf8parse::Parser,
}

impl FileSource {
    fn open<P: AsRef<Path>>(path: P) -> io::Result<FileSource> {
        File::open(&path).map(|file| {
            Self {
                name: path.as_ref().file_name().unwrap().to_string_lossy().into(),
                file: file,
                buffer: [0; 8192],
                len: 0,
                cursor: 0,
                parser: utf8parse::Parser::new(),
            }
        })
    }
}

impl Source for FileSource {
    fn name(&self) -> &str {
        &self.name
    }

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
                match self.file.read(&mut self.buffer)? {
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
