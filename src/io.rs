//! Standard IO streams and pipe management.
//!
//! Crush handles redirections and piping within the same process for functions and builtins using a context object,
//! which holds handles to stdin, stdout, and stderr.
use nix::unistd;
use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::*;
use termion;



/// A readable pipe. This is the type used for stdin.
pub struct ReadPipe(File);

impl ReadPipe {
    /// Check if the input stream is a TTY.
    pub fn is_tty(&self) -> bool {
        termion::is_tty(&self.0)
    }
}

impl Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Clone for ReadPipe {
    fn clone(&self) -> Self {
        ReadPipe(self.0.try_clone().expect("failed to duplicate pipe"))
    }
}


/// A writable pipe. This is the type used for stdout and stderr.
pub struct WritePipe(File);

impl Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl AsRawFd for WritePipe {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for WritePipe {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Clone for WritePipe {
    fn clone(&self) -> Self {
        WritePipe(self.0.try_clone().expect("failed to duplicate pipe"))
    }
}


/// An IO context.
pub struct IO {
    name: Cow<'static, str>,
    pub stdin: ReadPipe,
    pub stdout: WritePipe,
    pub stderr: WritePipe,
}

impl IO {
    /// Create an IO context from the process inherited streams.
    pub fn inherited() -> Self {
        unsafe {
            Self::new(
                "<stdin>",
                ReadPipe(File::from_raw_fd(0)),
                WritePipe(File::from_raw_fd(1)),
                WritePipe(File::from_raw_fd(2)),
            )
        }
    }

    /// Create a new IO context.
    pub fn new<S>(name: S, stdin: ReadPipe, stdout: WritePipe, stderr: WritePipe) -> Self
        where S: Into<Cow<'static, str>>
    {
        Self {
            name: name.into(),
            stdin: stdin,
            stdout: stdout,
            stderr: stderr,
        }
    }

    /// Get a name for the context based on the input stream, suitable for display purposes.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if the input stream is a TTY.
    pub fn is_tty(&self) -> bool {
        termion::is_tty(&self.stdin.0)
    }

    /// Split the IO context into two new contexts.
    ///
    /// The streams are set up as follows:
    /// - The first context inherits the previous stdin and stderr.
    /// - The second context inherits the previous stdout and stderr.
    /// - The first context's stdout and the second context's stdin form a new pipe.
    pub fn split(self) -> (Self, Self) {
        let head_stdin = self.stdin;
        let head_stderr = self.stderr.clone();

        let tail_stdout = self.stdout;
        let tail_stderr = self.stderr;

        let (head_stdout, tail_stdin) = pipe();

        (
            Self::new(self.name.clone(), head_stdin, head_stdout, head_stderr),
            Self::new(self.name.clone(), tail_stdin, tail_stdout, tail_stderr),
        )
    }

    /// Turn the IO context into a series of contexts piped together of the given length.
    pub fn pipeline(self, length: u16) -> Vec<Self> {
        let mut contexts = Vec::new();

        // If length is zero, just return empty.
        if length == 0 {
            return contexts;
        }

        let mut io_tail = self;

        for _ in 1..length {
            let split = io_tail.split();
            contexts.push(split.0);
            io_tail = split.1;
        }

        contexts.push(io_tail);
        contexts
    }
}

impl Clone for IO {
    fn clone(&self) -> Self {
        Self::new(
            self.name.clone(),
            self.stdin.clone(),
            self.stdout.clone(),
            self.stderr.clone(),
        )
    }
}


/// Create a new IO pipe.
pub fn pipe() -> (WritePipe, ReadPipe) {
    let fds = unistd::pipe().unwrap();

    unsafe {
        (WritePipe(File::from_raw_fd(fds.1)), ReadPipe(File::from_raw_fd(fds.0)))
    }
}
