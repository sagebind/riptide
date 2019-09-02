//! File descriptor and pipe utilities.

use super::io::{ReadHandle, RegisterRead, RegisterWrite, WriteHandle};
use futures::io::{AsyncRead, AsyncWrite};
use nix::{fcntl::OFlag, unistd::pipe2};
use std::{
    fmt,
    fs::File,
    io,
    io::Write,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    pin::Pin,
    task::{Context, Poll, Waker},
};

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
pub struct PipeReader {
    handle: Option<ReadHandle>,
    file: File,
}

impl PipeReader {
    pub fn stdin() -> Self {
        unsafe { Self::from_raw_fd(0) }
    }
}

impl FromRawFd for PipeReader {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            handle: None,
            file: File::from_raw_fd(fd),
        }
    }
}

impl AsRawFd for PipeReader {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl IntoRawFd for PipeReader {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

impl RegisterRead for PipeReader {
    fn init_read_handle(&mut self, handle: ReadHandle) {
        self.handle = Some(handle);
    }
}

impl AsyncRead for PipeReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unimplemented!()
    }
}

impl fmt::Debug for PipeReader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PipeReader")
            .field(&self.as_raw_fd())
            .finish()
    }
}

/// Writing end of an asynchronous pipe.
pub struct PipeWriter(File);

impl PipeWriter {
    pub fn stdout() -> Self {
        unsafe { Self::from_raw_fd(1) }
    }

    pub fn stderr() -> Self {
        unsafe { Self::from_raw_fd(2) }
    }
}

impl PipeWriter {
    unsafe fn enter<F, R>(self: Pin<&mut Self>, f: F) -> Poll<io::Result<R>>
    where
        F: FnOnce(&mut File) -> io::Result<R>,
    {
        match f(&mut self.get_unchecked_mut().0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            }
            result => Poll::Ready(result),
        }
    }
}

impl FromRawFd for PipeWriter {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(File::from_raw_fd(fd))
    }
}

impl AsRawFd for PipeWriter {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for PipeWriter {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl AsyncWrite for PipeWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe { self.enter(|file| file.write(buf)) }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe { self.enter(|file| file.flush()) }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!()
    }
}

impl fmt::Debug for PipeWriter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PipeWriter")
            .field(&self.as_raw_fd())
            .finish()
    }
}