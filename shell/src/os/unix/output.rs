use crate::editor::command::Command;
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
    is_alt_buffer: bool,
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
            is_alt_buffer: false,
        })
    }

    pub fn enter_raw_mode(&mut self) -> impl Drop {
        struct RawGuard(RawFd, Termios);

        impl Drop for RawGuard {
            fn drop(&mut self) {
                termios::tcsetattr(self.0, 0, &self.1).unwrap();
            }
        }

        termios::tcsetattr(self.as_raw_fd(), 0, &self.raw_termios).unwrap();

        RawGuard(self.as_raw_fd(), self.normal_termios.clone())
    }
}

impl<O: AsyncWrite + AsRawFd + Unpin> TerminalOutput<O> {
    pub async fn command(&mut self, command: Command) -> io::Result<()> {
        match command {
            Command::Clear => self.write_all(b"\x1b[2J").await,
            Command::ClearAfterCursor => self.write_all(b"\x1b[J").await,
            Command::MoveCursorLeft(n) => self.write_all(format!("\x1b[{}D", n).as_bytes()).await,
            Command::MoveCursorToAbsolute(x, y) => self.write_all(format!("\x1b[{};{}H", x, y).as_bytes()).await,
            Command::EnableAlternateBuffer => {
                if !self.is_alt_buffer {
                    self.write_all(b"\x1b[1049h").await?;
                    self.is_alt_buffer = true;
                }
                Ok(())
            },
            Command::DisableAlternateBuffer => {
                if self.is_alt_buffer {
                    self.write_all(b"\x1b[1049l").await?;
                    self.is_alt_buffer = false;
                }
                Ok(())
            }
        }
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
        let _ = termios::tcsetattr(self.as_raw_fd(), 0, &self.normal_termios);
    }
}
