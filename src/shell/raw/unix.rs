use crate::shell::event::Event;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{
    collections::VecDeque,
    io::{self, Read, Stdin, Stdout, Write},
    os::unix::io::{AsRawFd, RawFd},
};
use termios::Termios;

pub struct TerminalInput<I> {
    stdin: I,
    events: VecDeque<Event>,
    parser: vte::Parser,
}

impl TerminalInput<Stdin> {
    pub fn stdin() -> Self {
        Self::new(io::stdin())
    }
}

impl<I> TerminalInput<I> {
    pub fn new(stdin: I) -> Self {
        Self {
            stdin,
            events: VecDeque::default(),
            parser: vte::Parser::new(),
        }
    }

    fn parse_input(&mut self, byte: u8) {
        struct Perform<'a> {
            events: &'a mut VecDeque<Event>,
        }

        impl<'a> vte::Perform for Perform<'a> {
            fn print(&mut self, c: char) {
                self.events.push_back(match c {
                    '\x7f' => Event::Backspace,
                    c => Event::Char(c),
                });
            }

            fn execute(&mut self, byte: u8) {
                let event = match byte {
                    0 => Some(Event::Char('\0')),
                    b'\r' | b'\n' => Some(Event::Char('\n')),
                    0x01..=0x1a => Some(Event::Ctrl((byte - 0x01 + b'a') as char)),
                    0x1c..=0x1f => Some(Event::Ctrl((byte - 0x1c + b'4') as char)),
                    _ => None,
                };

                if let Some(event) = event {
                    self.events.push_back(event);
                } else {
                    log::debug!("unknown character: {}", byte);
                }
            }

            fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool) {
                log::info!("HOOK {:?} / {:?} / {}", params, intermediates, ignore);
            }

            fn put(&mut self, byte: u8) {
                log::debug!("PUT: {:?}", byte);
            }

            fn unhook(&mut self) {}

            fn osc_dispatch(&mut self, params: &[&[u8]]) {
                log::debug!("OSC: {:?}", params);
            }

            fn csi_dispatch(
                &mut self,
                params: &[i64],
                intermediates: &[u8],
                ignore: bool,
                c: char
            ) {
                match (c, params) {
                    ('A', _) => self.events.push_back(Event::Up),
                    ('B', _) => self.events.push_back(Event::Down),
                    ('C', _) => self.events.push_back(Event::Right),
                    ('D', _) => self.events.push_back(Event::Left),
                    ('F', _) | ('~', [4]) | ('~', [8]) => self.events.push_back(Event::End),
                    ('H', _) | ('~', [1]) | ('~', [7]) => self.events.push_back(Event::Home),
                    ('~', [2]) => self.events.push_back(Event::Insert),
                    ('~', [3]) => self.events.push_back(Event::Delete),
                    ('~', [5]) => self.events.push_back(Event::PageUp),
                    ('~', [6]) => self.events.push_back(Event::PageDown),
                    _ => log::info!("CSI {:?} / {:?} / {} / {}", params, intermediates, ignore, c),
                }
            }

            fn esc_dispatch(
                &mut self,
                params: &[i64],
                intermediates: &[u8],
                ignore: bool,
                byte: u8
            ) {
                log::info!("ESC {:?} / {:?} / {} / {}", params, intermediates, ignore, byte);
            }
        }

        let mut perform = Perform {
            events: &mut self.events,
        };

        self.parser.advance(&mut perform, byte);
    }
}

impl<I: AsyncRead + Unpin> TerminalInput<I> {
    pub async fn next_event_async(&mut self) -> io::Result<Event> {
        let mut buf = [0; 1024];

        loop {
            // If there's at least 1 pending event, return it.
            if let Some(event) = self.events.pop_front() {
                return Ok(event);
            }

            // Grab some more input.
            let count = self.stdin.read(&mut buf).await?;

            // Parse any events from the input if any.
            for i in 0..count {
                self.parse_input(buf[i]);
            }
        }
    }
}

impl<I: Read> TerminalInput<I> {
    pub fn next_event_blocking(&mut self) -> io::Result<Event> {
        let mut buf = [0; 1024];

        loop {
            // If there's at least 1 pending event, return it.
            if let Some(event) = self.events.pop_front() {
                return Ok(event);
            }

            // Grab some more input.
            let count = self.stdin.read(&mut buf)?;

            // Parse any events from the input if any.
            for i in 0..count {
                self.parse_input(buf[i]);
            }
        }
    }
}

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
            termios::tcsetattr(self.stdout.as_raw_fd(), 0, &self.raw_termios)
        } else {
            termios::tcsetattr(self.stdout.as_raw_fd(), 0, &self.normal_termios)
        }
    }

}

impl<O: AsRawFd + Write> Write for TerminalOutput<O> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl<O: AsRawFd> Drop for TerminalOutput<O> {
    fn drop(&mut self) {
        self.set_raw_mode(false).ok();
    }
}
