//! Standard IO streams and pipe management.
//!
//! Crush handles redirections and piping within the same process for functions and builtins using a context object,
//! which holds handles to stdin, stdout, and stderr.
use nix::unistd;
use std::borrow::Cow;
use std::fs::File;
use std::os::unix::io::*;
use termion;


pub struct IO {
    name: Cow<'static, str>,
    pub stdin: File,
    pub stdout: File,
    pub stderr: File,
}

impl IO {
    /// Create an IO context from the process inherited streams.
    pub fn inherited() -> Self {
        unsafe {
            Self::new(
                "<stdin>",
                File::from_raw_fd(0),
                File::from_raw_fd(1),
                File::from_raw_fd(2),
            )
        }
    }

    /// Create a new IO context.
    pub fn new<S>(name: S, stdin: File, stdout: File, stderr: File) -> Self
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
        termion::is_tty(&self.stdin)
    }
}

impl Clone for IO {
    fn clone(&self) -> Self {
        unsafe {
            let stdin = unistd::dup(self.stdin.as_raw_fd()).unwrap();
            let stdout = unistd::dup(self.stdout.as_raw_fd()).unwrap();
            let stderr = unistd::dup(self.stderr.as_raw_fd()).unwrap();

            Self::new(
                self.name.clone(),
                File::from_raw_fd(stdin),
                File::from_raw_fd(stdout),
                File::from_raw_fd(stderr),
            )
        }
    }
}

/// Create a new IO pipe.
pub fn pipe() -> (File, File) {
    let fds = unistd::pipe().unwrap();

    unsafe {
        (File::from_raw_fd(fds.0), File::from_raw_fd(fds.1))
    }
}
