//! Asynchronous I/O registration interfaces.
//!
//! This module defines the interface used by pipes and other file descriptors
//! for communicating with the reactor.

use nix::{fcntl::OFlag, unistd::pipe2};
use std::{
    fmt,
    fs::File,
    io,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{
    AsyncRead,
    AsyncWrite,
    PollEvented,
};

pub mod process;

/// An I/O context encapsulates the management of standard streams independently
/// of the current process, which allows more than one I/O context to coexist
/// inside the same process. This is essential in order to implement I/O aware
/// fibers.
pub struct IoContext {
    pub stdin: PipeReader,
    pub stdout: PipeWriter,
    pub stderr: PipeWriter,
}

impl IoContext {
    /// Create a new context inherited from the standard streams of the current
    /// OS process.
    pub fn from_process() -> io::Result<Self> {
        Ok(Self {
            stdin: unsafe { PipeReader::from_raw_fd(dup(io::stdin())?) },
            stdout: unsafe { PipeWriter::from_raw_fd(dup(io::stdout())?) },
            stderr: unsafe { PipeWriter::from_raw_fd(dup(io::stderr())?) },
        })
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
        let (r, w) = pipe()?;

        Ok((
            Self {
                stdin: self.stdin,
                stdout: w,
                stderr: self.stderr.try_clone()?,
            },
            Self {
                stdin: r,
                stdout: self.stdout,
                stderr: self.stderr,
            },
        ))
    }
}

/// Open a new pipe and return a reader/writer pair.
pub fn pipe() -> io::Result<(PipeReader, PipeWriter)> {
    pipe2(OFlag::O_CLOEXEC | OFlag::O_NONBLOCK)
        .map_err(nix_err)
        .map(|(read_fd, write_fd)| unsafe {
            (
                PipeReader::from_raw_fd(read_fd),
                PipeWriter::from_raw_fd(write_fd),
            )
        })
}

fn dup(file: impl AsRawFd) -> io::Result<RawFd> {
    nix::unistd::dup(file.as_raw_fd()).map_err(nix_err)
}

/// Reading end of an asynchronous pipe.
#[derive(Debug)]
pub struct PipeReader(PollEvented<EventedFd>);

impl PipeReader {
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.get_ref()
            .try_clone()
            .and_then(PollEvented::new)
            .map(PipeReader)
    }
}

impl From<PipeReader> for std::process::Stdio {
    fn from(pipe: PipeReader) -> Self {
        unsafe {
            Self::from_raw_fd(pipe.0.into_inner().unwrap().0.into_raw_fd())
        }
    }
}

impl FromRawFd for PipeReader {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(PollEvented::new(EventedFd::from_raw_fd(fd)).unwrap())
    }
}

impl AsRawFd for PipeReader {
    fn as_raw_fd(&self) -> RawFd {
        self.0.get_ref().as_raw_fd()
    }
}

impl AsyncRead for PipeReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            Pin::new_unchecked(&mut self.0).poll_read(cx, buf)
        }
    }
}

/// Writing end of an asynchronous pipe.
#[derive(Debug)]
pub struct PipeWriter(PollEvented<EventedFd>);

impl PipeWriter {
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.get_ref()
            .try_clone()
            .and_then(PollEvented::new)
            .map(PipeWriter)
    }
}

impl From<PipeWriter> for std::process::Stdio {
    fn from(pipe: PipeWriter) -> Self {
        unsafe {
            Self::from_raw_fd(pipe.0.into_inner().unwrap().0.into_raw_fd())
        }
    }
}

impl FromRawFd for PipeWriter {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(PollEvented::new(EventedFd::from_raw_fd(fd)).unwrap())
    }
}

impl AsRawFd for PipeWriter {
    fn as_raw_fd(&self) -> RawFd {
        self.0.get_ref().as_raw_fd()
    }
}

impl AsyncWrite for PipeWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            Pin::new_unchecked(&mut self.0).poll_write(cx, buf)
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe {
            Pin::new_unchecked(&mut self.0).poll_flush(cx)
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe {
            Pin::new_unchecked(&mut self.0).poll_shutdown(cx)
        }
    }
}

struct EventedFd(File);

impl EventedFd {
    fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(EventedFd)
    }
}

impl FromRawFd for EventedFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(File::from_raw_fd(fd))
    }
}

impl AsRawFd for EventedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl fmt::Debug for EventedFd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("EventedFd")
            .field(&self.as_raw_fd())
            .finish()
    }
}

impl mio::Evented for EventedFd {
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).deregister(poll)
    }
}

impl io::Read for EventedFd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl io::Write for EventedFd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

fn nix_err(error: nix::Error) -> io::Error {
    if let nix::Error::Sys(err_no) = error {
        io::Error::from(err_no)
    } else {
        io::Error::new(io::ErrorKind::Other, error)
    }
}
