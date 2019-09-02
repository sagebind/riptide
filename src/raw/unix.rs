use std::io::{self, Read, Stdin, Stdout, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use termion::event::Key; // remove
use termios::Termios;

/// A wrapper for a raw terminal.
// TODO: Change from sync to async operations.
pub struct RawTerminal<I: AsRawFd, O: AsRawFd> {
    stdin: I,
    stdout: O,
    parser: vte::Parser,
    normal_termios: Termios,
    raw_termios: Termios,
}

impl RawTerminal<Stdin, Stdout> {
    pub fn stdio() -> io::Result<Self> {
        Self::new(io::stdin(), io::stdout())
    }
}

impl<I: AsRawFd, O: AsRawFd> RawTerminal<I, O> {
    pub fn new(stdin: I, stdout: O) -> io::Result<Self> {
        let normal_termios = Termios::from_fd(stdin.as_raw_fd())?;
        let mut raw_termios = normal_termios;
        termios::cfmakeraw(&mut raw_termios);

        Ok(Self {
            stdin,
            stdout,
            parser: vte::Parser::new(),
            normal_termios,
            raw_termios,
        })
    }

    pub fn set_raw_mode(&mut self, raw: bool) -> io::Result<()> {
        if raw {
            termios::tcsetattr(self.stdin.as_raw_fd(), 0, &self.raw_termios)
        } else {
            termios::tcsetattr(self.stdin.as_raw_fd(), 0, &self.normal_termios)
        }
    }
}

impl<I: AsRawFd + Read, O: AsRawFd> RawTerminal<I, O> {
    pub fn next_input_blocking(&mut self) -> io::Result<()> {
        let mut buf = [0; 16];

        struct Queue {
            keys: Vec<Key>,
        }

        impl vte::Perform for Queue {
            fn print(&mut self, c: char) {
                self.keys.push(Key::Char(c));
            }

            fn execute(&mut self, byte: u8) {}

            fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool) {}

            fn put(&mut self, byte: u8) {}

            fn unhook(&mut self) {}

            fn osc_dispatch(&mut self, params: &[&[u8]]) {}

            fn csi_dispatch(
                &mut self,
                params: &[i64],
                intermediates: &[u8],
                ignore: bool,
                c: char
            ) {
                log::info!("{:?} / {:?} / {} / {}", params, intermediates, ignore, c);
            }

            fn esc_dispatch(
                &mut self,
                params: &[i64],
                intermediates: &[u8],
                ignore: bool,
                byte: u8
            ) {}
        }

        loop {
            let count = self.stdin.read(&mut buf)?;
            break;
        }

        Ok(())
    }
}

impl<I: AsRawFd, O: AsRawFd + Write> Write for RawTerminal<I, O> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl<I: AsRawFd, O: AsRawFd> Drop for RawTerminal<I, O> {
    fn drop(&mut self) {
        self.set_raw_mode(false).ok();
    }
}
