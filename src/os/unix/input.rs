use crate::editor::event::Event;
use std::{
    collections::VecDeque,
    io,
};
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct TerminalInput<I> {
    stdin: I,
    events: VecDeque<Event>,
    parser: vte::Parser,
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
                    0 => Some(Event::Eof),
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

            fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, action: char) {
                log::info!("HOOK {:?} / {:?} / {} / {}", params, intermediates, ignore, action);
            }

            fn put(&mut self, byte: u8) {
                log::debug!("PUT: {:?}", byte);
            }

            fn unhook(&mut self) {}

            fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
                log::debug!("OSC: {:?} / {}", params, bell_terminated);
            }

            fn csi_dispatch(
                &mut self,
                params: &[i64],
                intermediates: &[u8],
                ignore: bool,
                action: char
            ) {
                match (action, params) {
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
                    _ => log::info!("CSI {:?} / {:?} / {} / {}", params, intermediates, ignore, action),
                }
            }

            fn esc_dispatch(
                &mut self,
                intermediates: &[u8],
                ignore: bool,
                byte: u8
            ) {
                log::info!("ESC {:?} / {} / {}", intermediates, ignore, byte);
            }
        }

        let mut perform = Perform {
            events: &mut self.events,
        };

        self.parser.advance(&mut perform, byte);
    }
}

impl<I: AsyncRead + Unpin> TerminalInput<I> {
    pub async fn next_event(&mut self) -> io::Result<Event> {
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
