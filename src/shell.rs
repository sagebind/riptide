use std::borrow::Cow;
use std::fs::File;
use std::os::unix::io::*;
use termion;


/// Open the stream set as file descriptor 0.
pub fn stdin() -> File {
    unsafe {
        File::from_raw_fd(0)
    }
}

/// Open the stream set as file descriptor 1.
pub fn stdout() -> File {
    unsafe {
        File::from_raw_fd(1)
    }
}

/// Open the stream set as file descriptor 2.
pub fn stderr() -> File {
    unsafe {
        File::from_raw_fd(2)
    }
}


/// Encapsulates the internal represenation of a shell session. Under certain circumstances, more than one of these may
/// exist inside a single process.
pub struct Shell {
    filename: Cow<'static, str>,
    pub stdin: File,
    pub stdout: File,
    pub stderr: File,
}

impl Shell {
    /// Create a shell instance from the current process.
    ///
    /// The current standard file descriptors will be opened and used as this shell's input, output, and error streams.
    pub fn current() -> Self {
        Shell::new("<stdin>", stdin(), stdout(), stderr())
    }

    /// Create a new shell instance.
    pub fn new<S>(name: S, stdin: File, stdout: File, stderr: File) -> Self
        where S: Into<Cow<'static, str>>
    {
        Self {
            filename: name.into(),
            stdin: stdin,
            stdout: stdout,
            stderr: stderr,
        }
    }

    /// Get a name for the shell based on the input stream, suitable for display purposes.
    #[inline]
    pub fn name(&self) -> &str {
        &self.filename
    }

    /// Check if the input stream is a TTY.
    pub fn is_tty(&self) -> bool {
        termion::is_tty(&self.stdin)
    }
}
