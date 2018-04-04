use prompt::Prompt;
use riptide::fd::*;
use std::borrow::Cow;
use std::cmp;
use std::fs::File;
use std::io::{self, Write};
use std::mem::swap;
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
    prompt: Prompt,
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
            prompt: Prompt::new(),
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
                    self.prompt.move_cursor_relative(-1);
                    self.redraw_needed();
                },
                Key::Right => {
                    self.prompt.move_cursor_relative(1);
                    self.redraw_needed();
                },
                Key::Home => {
                    self.prompt.move_to_start_of_line();
                    self.redraw_needed();
                },
                Key::End => {
                    self.prompt.move_to_end_of_line();
                    self.redraw_needed();
                },
                Key::Char(c) => {
                    self.prompt.insert_char(c);
                    self.redraw_needed();
                },
                Key::Backspace => {
                    self.prompt.delete_before_cursor();
                    self.redraw_needed();
                },
                Key::Delete => {
                    self.prompt.delete_after_cursor();
                    self.redraw_needed();
                },
                Key::Ctrl('c') => {
                    self.prompt.clear();
                    self.redraw_needed();
                },
                _ => {},
            }

            // if *exit::flag() {
            //     break;
            // }

            self.redraw_if_needed();
        }

        // Move the command line out of out buffer and return it.
        self.prompt.take_text()
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
        let prompt = self.get_prompt_str();
        write!(self.stdout, "\r{}{}{}", clear::AfterCursor, prompt, self.prompt.text()).unwrap();

        // Update the cursor position.
        let diff = self.prompt.text().len() - self.prompt.cursor();
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
