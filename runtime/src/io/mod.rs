//! Asynchronous I/O registration interfaces.
//!
//! This module defines the interface used by pipes and other file descriptors
//! for communicating with the reactor.

use std::{
    fmt,
    io,
    marker::Unpin,
    os::unix::io::{AsRawFd, FromRawFd, RawFd},
    process::Stdio,
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncWrite, Stderr, Stdin, Stdout},
};
use tokio_pipe::{pipe, PipeRead, PipeWrite};

pub mod process;
mod unix;

/// An I/O context encapsulates the management of standard streams independently
/// of the current process, which allows more than one I/O context to coexist
/// inside the same process. This is essential in order to implement I/O aware
/// fibers.
pub struct IoContext {
    stdin: Box<dyn Input>,
    stdout: Box<dyn Output>,
    stderr: Box<dyn Output>,
}

impl IoContext {
    /// Create a new context inherited from the standard streams of the current
    /// OS process.
    pub fn from_process() -> io::Result<Self> {
        Ok(Self {
            stdin: Box::new({
                let mut stdin = unix::dup::<_, PipeRead>(tokio::io::stdin())?;

                unix::set_nonblocking(&mut stdin, true)?;

                stdin
            }),
            stdout: Box::new(unix::dup::<_, PipeWrite>(tokio::io::stdout())?),
            stderr: Box::new(unix::dup::<_, PipeWrite>(tokio::io::stderr())?),
            // stdin: Box::new(tokio::io::stdin()),
            // stdout: Box::new(tokio::io::stdout()),
            // stderr: Box::new(tokio::io::stderr()),
        })
    }

    pub fn stdin(&mut self) -> &mut dyn Input {
        &mut *self.stdin
    }

    pub fn stdout(&mut self) -> &mut dyn Output {
        &mut *self.stdout
    }

    pub fn stderr(&mut self) -> &mut dyn Output {
        &mut *self.stderr
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            stdin: self.stdin.try_clone()?,
            stdout: self.stdout.try_clone()?,
            stderr: self.stderr.try_clone()?,
        })
    }

    /// Split this context in half, returning two new contexts that have their
    /// standard output and standard input connected with a pipe.
    pub fn split(self) -> io::Result<(IoContext, IoContext)> {
        let (stdin, stdout) = pipe()?;

        Ok((
            Self {
                stdin: self.stdin,
                stdout: Box::new(stdout),
                stderr: self.stderr.try_clone()?,
            },
            Self {
                stdin: Box::new(stdin),
                stdout: self.stdout,
                stderr: self.stderr,
            },
        ))
    }

    pub fn split_n(self, n: usize) -> io::Result<Vec<IoContext>> {
        let mut ios = Vec::new();
        let mut next = Some(self);

        for i in 0..n {
            if i == n - 1 {
                ios.push(next.take().unwrap());
            } else {
                let (left, right) = next.take().unwrap().split()?;
                next = Some(right);
                ios.push(left);
            }
        }

        Ok(ios)
    }
}

impl fmt::Debug for IoContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IoContext").finish()
    }
}

/// An I/O input port.
pub trait Input: AsyncRead + AsRawFd + Unpin + Send {
    fn try_clone(&self) -> io::Result<Box<dyn Input>>;

    /// Create a synchronous clone of this file descriptor for piping with
    /// external processes.
    fn create_stdio(&self) -> io::Result<Stdio> {
        unix::dup(self.as_raw_fd())
    }

    /// Enable or disable non-blocking mode on this file descriptor. This
    /// affects all clones of this file descriptor as well, so use with caution.
    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        unix::set_nonblocking(&mut self.as_raw_fd(), nonblocking)
    }
}

impl AsRawFd for Box<dyn Input> {
    fn as_raw_fd(&self) -> RawFd {
        (**self).as_raw_fd()
    }
}

impl Input for Stdin {
    fn try_clone(&self) -> io::Result<Box<dyn Input>> {
        Ok(Box::new(tokio::io::stdin()))
    }

    fn set_nonblocking(&mut self, _nonblocking: bool) -> io::Result<()> {
        // Tokio implementation is already blocking, enabling non-blocking is
        // not needed and also breaks stuff.
        Ok(())
    }
}

impl Input for PipeRead {
    fn try_clone(&self) -> io::Result<Box<dyn Input>> {
        Ok(Box::new(unix::dup::<_, Self>(self.as_raw_fd())?))
    }
}

impl Input for File {
    fn try_clone(&self) -> io::Result<Box<dyn Input>> {
        Ok(Box::new(unix::dup::<_, Self>(self.as_raw_fd())?))
    }
}

/// An I/O output port.
pub trait Output: AsyncWrite + AsRawFd + Unpin + Send {
    fn try_clone(&self) -> io::Result<Box<dyn Output>>;

    /// Create a synchronous clone of this file descriptor for piping with
    /// external processes.
    fn create_stdio(&self) -> io::Result<Stdio> {
        let fd = unix::dup(self.as_raw_fd())?;

        Ok(unsafe { Stdio::from_raw_fd(fd) })
    }

    /// Enable or disable non-blocking mode on this file descriptor. This
    /// affects all clones of this file descriptor as well, so use with caution.
    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        unix::set_nonblocking(&mut self.as_raw_fd(), nonblocking)
    }
}

impl AsRawFd for Box<dyn Output> {
    fn as_raw_fd(&self) -> RawFd {
        (**self).as_raw_fd()
    }
}

impl Output for Stdout {
    fn try_clone(&self) -> io::Result<Box<dyn Output>> {
        Ok(Box::new(tokio::io::stdout()))
    }

    fn set_nonblocking(&mut self, _nonblocking: bool) -> io::Result<()> {
        // Tokio implementation is already blocking, enabling non-blocking is
        // not needed and also breaks stuff.
        Ok(())
    }
}

impl Output for Stderr {
    fn try_clone(&self) -> io::Result<Box<dyn Output>> {
        Ok(Box::new(tokio::io::stderr()))
    }

    fn set_nonblocking(&mut self, _nonblocking: bool) -> io::Result<()> {
        // Tokio implementation is already blocking, enabling non-blocking is
        // not needed and also breaks stuff.
        Ok(())
    }
}

impl Output for PipeWrite {
    fn try_clone(&self) -> io::Result<Box<dyn Output>> {
        Ok(Box::new(unix::dup::<_, Self>(self.as_raw_fd())?))
    }
}

impl Output for File {
    fn try_clone(&self) -> io::Result<Box<dyn Output>> {
        Ok(Box::new(unix::dup::<_, Self>(self.as_raw_fd())?))
    }
}
