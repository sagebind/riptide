use buffer::Buffer;
use riptide::fd::*;
use std::borrow::Cow;
use std::io::{self, Write};
use std::os::unix::io::*;
use termion::clear;
use termion::cursor;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::*;

/// The default prompt string if none is defined.
const DEFAULT_PROMPT: &str = "$ ";

/// Controls the interactive command line editor.
pub struct Editor {
    stdin: ReadPipe,
    stdout: WritePipe,
    stderr: WritePipe,
    buffer: Buffer,
    // Whether the command line needs redrawn.
    redraw_needed: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            stdin: unsafe {
                ReadPipe::from_raw_fd(0)
            },
            stdout: unsafe {
                WritePipe::from_raw_fd(0)
            },
            stderr: unsafe {
                WritePipe::from_raw_fd(0)
            },
            buffer: Buffer::new(),
            redraw_needed: false,
        }
    }

    pub fn read_line(&mut self) -> String {
        let prompt = self.get_prompt_str();
        write!(self.stdout, "{}", prompt);
        self.stdout.flush().unwrap();

        // Duplicate stdin and stdout handles to workaround Termion's API.
        let stdin = self.stdin.clone();
        let stdout = self.stdout.clone();

        // Enter raw mode.
        let _raw_guard = stdout.into_raw_mode().unwrap();

        // Handle keyboard events.
        for key in stdin.keys() {
            match key.unwrap() {
                Key::Char('\n') => {
                    write!(self.stdout, "\r\n").unwrap();
                    break;
                }
                Key::Left => {
                    self.buffer.move_cursor_relative(-1);
                    self.redraw_needed();
                },
                Key::Right => {
                    self.buffer.move_cursor_relative(1);
                    self.redraw_needed();
                },
                Key::Home => {
                    self.buffer.move_to_start_of_line();
                    self.redraw_needed();
                },
                Key::End => {
                    self.buffer.move_to_end_of_line();
                    self.redraw_needed();
                },
                Key::Char(c) => {
                    self.buffer.insert_char(c);
                    self.redraw_needed();
                },
                Key::Backspace => {
                    self.buffer.delete_before_cursor();
                    self.redraw_needed();
                },
                Key::Delete => {
                    self.buffer.delete_after_cursor();
                    self.redraw_needed();
                },
                Key::Ctrl('c') => {
                    self.buffer.clear();
                    self.redraw_needed();
                },
                _ => {},
            }

            self.redraw_if_needed();
        }

        // Move the command line out of out buffer and return it.
        self.buffer.take_text()
    }

    /// Signal that the prompt needs to be redrawn.
    pub fn redraw_needed(&mut self) {
        self.redraw_needed = true;
    }

    /// Redraw the prompt if it is needed.
    pub fn redraw_if_needed(&mut self) {
        if self.redraw_needed {
            self.redraw();
        }
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

        self.redraw_needed = false;
    }

    fn get_prompt_str(&self) -> Cow<'static, str> {
        // match interpreter::function_call(PROMPT_FUNCTION, &[], &mut Streams::null()) {
        //     Ok(Expression::Atom(s)) => s,
        //     _ => Cow::Borrowed(DEFAULT_PROMPT),
        // }

        Cow::Borrowed(DEFAULT_PROMPT)
    }
}
