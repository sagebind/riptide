use super::buffer::Buffer;
use crate::shell::{
    event::Event,
    raw::TerminalInput,
};
use std::borrow::Cow;
use std::io::{self, Read, Write};
use termion::{
    clear,
    cursor,
    event::Key,
    input::{
        Keys,
        TermRead,
    },
    raw::{
        IntoRawMode,
        RawTerminal,
    },
};

/// The default prompt string if none is defined.
const DEFAULT_PROMPT: &str = "$ ";

/// Controls the interactive command line editor.
pub struct Editor<I: Read, O: Write> {
    // stdin: Keys<I>,
    stdin: TerminalInput<I>,
    stdout: RawTerminal<O>,
    buffer: Buffer,
}

impl Default for Editor<io::Stdin, io::Stdout> {
    fn default() -> Self {
        Self::new(io::stdin(), io::stdout())
    }
}

impl<I: Read, O: Write> Editor<I, O> {
    pub fn new(stdin: I, stdout: O) -> Self {
        Self {
            stdin: TerminalInput::new(stdin),
            stdout: stdout.into_raw_mode().unwrap(),
            buffer: Buffer::new(),
        }
    }

    /// Show a command prompt to the user and await for the user to input a
    /// command. The typed command is returned once submitted.
    pub fn read_line(&mut self) -> String {
        let prompt = self.get_prompt_str();
        write!(self.stdout, "{}", prompt).unwrap();
        self.stdout.flush().unwrap();

        // Enter raw mode.
        self.stdout.activate_raw_mode().unwrap();

        // Handle keyboard events.
        while let Ok(event) = self.stdin.next_event_blocking() {
            match event {
                Event::Char('\n') => {
                    write!(self.stdout, "\r\n").unwrap();
                    break;
                }
                Event::Left => {
                    self.buffer.move_cursor_relative(-1);
                }
                Event::Right => {
                    self.buffer.move_cursor_relative(1);
                }
                Event::Home => {
                    self.buffer.move_to_start_of_line();
                }
                Event::End => {
                    self.buffer.move_to_end_of_line();
                }
                Event::Char(c) => {
                    self.buffer.insert_char(c);
                }
                Event::Backspace => {
                    self.buffer.delete_before_cursor();
                }
                Event::Delete => {
                    self.buffer.delete_after_cursor();
                }
                Event::Ctrl('c') => {
                    self.buffer.clear();
                }
                _ => {}
            }

            self.redraw();
        }

        self.stdout.suspend_raw_mode().unwrap();

        // Move the command line out of out buffer and return it.
        self.buffer.take_text()
    }

    /// Redraw the buffer.
    pub fn redraw(&mut self) {
        let prompt = self.get_prompt_str();
        write!(self.stdout, "\r{}{}{}", clear::AfterCursor, prompt, self.buffer.text()).unwrap();

        // Update the cursor position.
        let diff = self.buffer.text().len() - self.buffer.cursor();
        if diff > 0 {
            write!(self.stdout, "{}", cursor::Left(diff as u16)).unwrap();
        }

        // Flush all changes from the IO buffer.
        self.stdout.flush().unwrap();
    }

    fn get_prompt_str(&self) -> Cow<'static, str> {
        // match interpreter::function_call(PROMPT_FUNCTION, &[], &mut Streams::null()) {
        //     Ok(Expression::Atom(s)) => s,
        //     _ => Cow::Borrowed(DEFAULT_PROMPT),
        // }

        Cow::Borrowed(DEFAULT_PROMPT)
    }
}
