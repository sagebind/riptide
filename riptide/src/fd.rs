//! File descriptor and pipe utilities.
use nix::unistd;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::*;

pub fn stdin() -> ReadPipe {
    unsafe {
        ReadPipe::from_raw_fd(0)
    }
}

pub fn stdout() -> WritePipe {
    unsafe {
        WritePipe::from_raw_fd(1)
    }
}

pub fn stderr() -> WritePipe {
    unsafe {
        WritePipe::from_raw_fd(1)
    }
}

/// Create a new IO pipe.
pub fn pipe() -> (WritePipe, ReadPipe) {
    let fds = unistd::pipe().unwrap();

    unsafe { (WritePipe::from_raw_fd(fds.1), ReadPipe::from_raw_fd(fds.0)) }
}

/// A readable pipe. This is the type used for stdin.
pub struct ReadPipe(File);

impl ReadPipe {
    /// Check if the input stream is a TTY.
    pub fn is_tty(&self) -> bool {
        match unistd::isatty(self.as_raw_fd()) {
            Ok(true) => true,
            _ => false,
        }
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

impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        ReadPipe(File::from_raw_fd(fd))
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

impl FromRawFd for WritePipe {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        WritePipe(File::from_raw_fd(fd))
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
