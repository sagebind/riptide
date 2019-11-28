//! File descriptor and pipe utilities.

use nix::{fcntl::OFlag, unistd::pipe2};
use std::{
    fmt,
    fs::File,
    io,
    os::unix::io::{AsRawFd, FromRawFd, RawFd},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{
    AsyncRead,
    AsyncWrite,
    PollEvented,
};

pub fn stdin() -> PipeReader {
    unsafe { PipeReader::from_raw_fd(0) }
}

pub fn stdout() -> PipeWriter {
    unsafe { PipeWriter::from_raw_fd(1) }
}

pub fn stderr() -> PipeWriter {
    unsafe { PipeWriter::from_raw_fd(2) }
}

/// Open a new pipe and return a reader/writer pair.
pub fn pipe() -> io::Result<(PipeReader, PipeWriter)> {
    pipe2(OFlag::O_CLOEXEC | OFlag::O_NONBLOCK)
        .map_err(|e| {
            if let nix::Error::Sys(err_no) = e {
                io::Error::from(err_no)
            } else {
                panic!("unexpected nix error type: {:?}", e)
            }
        })
        .map(|(read_fd, write_fd)| unsafe {
            (
                PipeReader::from_raw_fd(read_fd),
                PipeWriter::from_raw_fd(write_fd),
            )
        })
}

/// Reading end of an asynchronous pipe.
#[derive(Debug)]
pub struct PipeReader(PollEvented<EventedFd>);

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
