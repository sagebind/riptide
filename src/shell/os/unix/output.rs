use crate::shell::command::Command;
use futures::io::{AsyncWrite, AsyncWriteExt};
use std::{
    io,
    io::{Stdout, Write},
    os::unix::io::{AsRawFd, RawFd},
};
use termios::Termios;

pub struct TerminalOutput<O: AsRawFd> {
    stdout: O,
    normal_termios: Termios,
    raw_termios: Termios,
}

impl TerminalOutput<Stdout> {
    pub fn stdout() -> io::Result<Self> {
        Self::new(io::stdout())
    }
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

impl<O: Write + AsRawFd> TerminalOutput<O> {
    pub fn command_blocking(&mut self, command: Command) -> io::Result<()> {
        self.write_all(match command {
            Command::ClearAfterCursor => String::from("\x1b[J"),
            Command::MoveCursorLeft(n) => format!("\x1b[{}D", n),
        }.as_bytes())
    }
}

impl<O: Write + AsRawFd> Write for TerminalOutput<O> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
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
