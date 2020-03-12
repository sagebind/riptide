use super::buffer::Buffer;
use crate::{
    editor::command::Command,
    editor::event::Event,
    history::{EntryCursor, History},
    os::{TerminalInput, TerminalOutput},
};
use std::borrow::Cow;
use std::os::unix::io::AsRawFd;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

pub mod command;
pub mod event;

/// The default prompt string if none is defined.
const DEFAULT_PROMPT: &str = "$ ";

/// Controls the interactive command line editor.
pub struct Editor<I, O: AsRawFd> {
    stdin: TerminalInput<I>,
    stdout: TerminalOutput<O>,
    history: History,
    history_cursor: Option<EntryCursor>,
    buffer: Buffer,
}

impl<I, O: AsRawFd> Editor<I, O> {
    pub fn new(stdin: I, stdout: O, history: History) -> Self {
        Self {
            stdin: TerminalInput::new(stdin),
            stdout: TerminalOutput::new(stdout).unwrap(),
            history,
            history_cursor: None,
            buffer: Buffer::new(),
        }
    }

    fn get_prompt_str(&self) -> Cow<'static, str> {
        // match interpreter::function_call(PROMPT_FUNCTION, &[], &mut Streams::null()) {
        //     Ok(Expression::Atom(s)) => s,
        //     _ => Cow::Borrowed(DEFAULT_PROMPT),
        // }

        Cow::Borrowed(DEFAULT_PROMPT)
    }
}

impl<I: AsyncRead + Unpin, O: AsyncWrite + AsRawFd + Unpin> Editor<I, O> {
    /// Show a command prompt to the user and await for the user to input a
    /// command. The typed command is returned once submitted.
    pub async fn read_line(&mut self) -> String {
        let prompt = self.get_prompt_str();
        self.stdout.write_all(prompt.as_bytes()).await.unwrap();
        self.stdout.flush().await.unwrap();

        // Enter raw mode.
        self.stdout.set_raw_mode(true).unwrap();

        // Handle keyboard events.
        while let Ok(event) = self.stdin.next_event().await {
            match event {
                Event::Char('\n') => {
                    self.stdout.write_all(b"\r\n").await.unwrap();
                    break;
                }
                Event::Left => {
                    self.buffer.move_cursor_relative(-1);
                }
                Event::Right => {
                    self.buffer.move_cursor_relative(1);
                }
                Event::Up => {
                    let history = self.history.clone();

                    match self.history_cursor.get_or_insert_with(|| history.entries()).next() {
                        Some(entry) => {
                            // TODO: Save buffer for later if user wants to return to
                            // what they typed.
                            self.buffer.clear();
                            self.buffer.insert_str(entry.command());
                        }
                        None => {
                            // TODO
                        }
                    }
                }
                Event::Down => {
                    if let Some(mut cursor) = self.history_cursor.take() {
                        self.buffer.clear();

                        if let Some(entry) = cursor.prev() {
                            self.buffer.insert_str(entry.command());
                            self.history_cursor = Some(cursor);
                        }
                    }

                    // TODO: Restore original buffer
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

            self.redraw().await;
        }

        self.history_cursor = None;

        self.stdout.set_raw_mode(false).unwrap();

        // Record line to history.
        self.history.add(self.buffer.text());

        // Move the command line out of out buffer and return it.
        self.buffer.take_text()
    }

    /// Redraw the buffer.
    pub async fn redraw(&mut self) {
        let prompt = self.get_prompt_str();
        self.stdout.write_all(b"\r").await.unwrap();
        self.stdout
            .command(Command::ClearAfterCursor)
            .await
            .unwrap();
        self.stdout
            .write_all(format!("{}{}", prompt, self.buffer.text()).as_bytes())
            .await
            .unwrap();

        // Update the cursor position.
        let diff = self.buffer.text().len() - self.buffer.cursor();
        if diff > 0 {
            self.stdout
                .command(Command::MoveCursorLeft(diff))
                .await
                .unwrap();
        }

        // Flush all changes from the IO buffer.
        self.stdout.flush().await.unwrap();
    }
}
