use exit;
use io::IO;
use std::cmp;
use std::io::Write;
use std::mem::swap;
use termion::clear;
use termion::cursor;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::*;


const DEFAULT_PROMPT: &str = "$ ";

/// Controls the interactive command line editor.
pub struct Editor<'s> {
    io: &'s mut IO,
    // Current command line buffer.
    buffer: String,
    // Current cursor position in the buffer.
    cursor: usize,
    // Whether the command line needs redrawn.
    redraw_needed: bool,
}

impl<'s> Editor<'s> {
    pub fn new(io: &'s mut IO) -> Self {
        Self {
            io: io,
            buffer: String::new(),
            cursor: 0,
            redraw_needed: false,
        }
    }

    pub fn read_line(&mut self) -> String {
        write!(self.io.stdout, "{}", DEFAULT_PROMPT);
        self.io.stdout.flush().unwrap();

        // Duplicate stdin and stdout handles to workaround Termion's API.
        let stdin = self.io.stdin.try_clone().unwrap();
        let stdout = self.io.stdout.try_clone().unwrap();

        // Enter raw mode.
        let _raw_guard = stdout.into_raw_mode().unwrap();

        // Handle keyboard events.
        for key in stdin.keys() {
            match key.unwrap() {
                Key::Char('\n') => {
                    write!(self.io.stdout, "\r\n").unwrap();
                    break;
                }
                Key::Left => self.move_cursor_relative(-1),
                Key::Right => self.move_cursor_relative(1),
                Key::Home => self.move_to_start_of_line(),
                Key::End => self.move_to_end_of_line(),
                Key::Char(c) => self.insert_char_after_cursor(c),
                Key::Backspace => self.delete_before_cursor(),
                Key::Delete => self.delete_after_cursor(),
                Key::Ctrl('c') => self.clear_buffer(),
                _ => {},
            }

            if *exit::flag() {
                break;
            }

            self.redraw_if_needed();
        }

        // Move the command line out of out buffer and return it.
        let mut line = String::new();
        swap(&mut line, &mut self.buffer);
        line
    }

    pub fn move_cursor_to(&mut self, pos: usize) {
        if pos == self.cursor {
            return;
        }

        let pos = cmp::min(pos, self.buffer.len());

        if pos < self.cursor {
            write!(self.io.stdout, "{}", cursor::Left((self.cursor - pos) as u16)).unwrap();
            self.cursor = pos;
        } else if pos > self.cursor {
            write!(self.io.stdout, "{}", cursor::Right((pos - self.cursor) as u16)).unwrap();
            self.cursor = pos;
        }
    }

    pub fn move_cursor_relative(&mut self, offset: i32) {
        let pos = cmp::max(0, self.cursor as i32 + offset) as usize;
        self.move_cursor_to(pos);
    }

    pub fn move_to_start_of_line(&mut self) {
        self.cursor = 0;
        self.redraw_needed();
    }

    pub fn move_to_end_of_line(&mut self) {
        self.cursor = self.buffer.len();
        self.redraw_needed();
    }

    pub fn insert_char_after_cursor(&mut self, c: char) {
        self.buffer.insert(self.cursor as usize, c);
        self.cursor += 1;

        self.redraw_needed();
    }

    pub fn insert_str_after_cursor<S>(&mut self, s: S)
        where S: AsRef<str>
    {
        let s = s.as_ref();
        self.buffer.insert_str(self.cursor as usize, s);
        self.cursor += s.len();
        self.redraw_needed();
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.buffer.remove(self.cursor - 1);
            self.cursor = self.cursor.saturating_sub(1);

            self.redraw_needed();
        }
    }

    pub fn delete_after_cursor(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);

            self.redraw_needed();
        }
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.cursor = 0;

        self.redraw_needed();
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

    /// Redraw the prompt.
    pub fn redraw(&mut self) {
        write!(self.io.stdout, "\r{}{}{}", clear::AfterCursor, DEFAULT_PROMPT, self.buffer).unwrap();

        // Update the cursor position.
        let diff = self.buffer.len() - self.cursor;
        if diff > 0 {
            write!(self.io.stdout, "{}", cursor::Left(diff as u16)).unwrap();
        }

        // Flush all changes from the IO buffer.
        self.io.stdout.flush().unwrap();

        self.redraw_needed = false;
    }
}
