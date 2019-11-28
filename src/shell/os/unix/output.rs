use crate::shell::command::Command;
use std::{
    io,
    pin::Pin,
    os::unix::io::{AsRawFd, RawFd},
    task::{Context, Poll},
};
use termios::Termios;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct TerminalOutput<O: AsRawFd> {
    stdout: O,
    normal_termios: Termios,
    raw_termios: Termios,
}

impl<O: AsRawFd> TerminalOutput<O> {
    pub fn new(stdout: O) -> io::Result<Self> {
        let normal_termios = Termios::from_fd(stdout.as_raw_fd())?;
        let mut raw_termios = normal_termios;
        termios::cfmakeraw(&mut raw_termios);

        Ok(Self {
            stdout,
            normal_termios,
            raw_termios,
        })
    }

    pub fn set_raw_mode(&mut self, raw: bool) -> io::Result<()> {
        if raw {
            termios::tcsetattr(self.as_raw_fd(), 0, &self.raw_termios)
        } else {
            termios::tcsetattr(self.as_raw_fd(), 0, &self.normal_termios)
        }
    }
}

impl<O: AsyncWrite + AsRawFd + Unpin> TerminalOutput<O> {
    pub async fn command(&mut self, command: Command) -> io::Result<()> {
        self.write_all(match command {
            Command::ClearAfterCursor => String::from("\x1b[J"),
            Command::MoveCursorLeft(n) => format!("\x1b[{}D", n),
        }.as_bytes()).await
    }
}

impl<O: AsyncWrite + AsRawFd + Unpin> AsyncWrite for TerminalOutput<O> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, io::Error>> {
        unsafe {
            Pin::new_unchecked(&mut self.stdout).poll_write(cx, buf)
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        unsafe {
            Pin::new_unchecked(&mut self.stdout).poll_flush(cx)
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<Result<(), io::Error>> {
        unsafe {
            Pin::new_unchecked(&mut self.stdout).poll_shutdown(cx)
        }
    }
}

impl<O: AsRawFd> AsRawFd for TerminalOutput<O> {
    fn as_raw_fd(&self) -> RawFd {
        self.stdout.as_raw_fd()
    }
}

impl<O: AsRawFd> Drop for TerminalOutput<O> {
    fn drop(&mut self) {
        self.set_raw_mode(false).ok();
    }
}
