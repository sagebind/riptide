use super::buffer::Buffer;
use crate::{
    editor::command::Command,
    editor::event::Event,
    history::{EntryCursor, History, Session},
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
    history_session: Session,
    history_cursor: Option<EntryCursor>,
    buffer: Buffer,
}

pub enum ReadLine {
    Input(String),
    Eof,
}

impl<I, O: AsRawFd> Editor<I, O> {
    pub fn new(stdin: I, stdout: O, history: History, session: Session) -> Self {
        Self {
            stdin: TerminalInput::new(stdin),
            stdout: TerminalOutput::new(stdout).unwrap(),
            history,
            history_session: session,
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
    pub async fn read_line(&mut self) -> ReadLine {
        let prompt = self.get_prompt_str();
        self.stdout.write_all(prompt.as_bytes()).await.unwrap();
        self.stdout.flush().await.unwrap();

        let mut editor = scopeguard::guard(self, |editor| {
            editor.stdout.set_raw_mode(false).unwrap();
        });

        // Enter raw mode.
        editor.stdout.set_raw_mode(true).unwrap();

        // Handle keyboard events.
        while let Ok(event) = editor.stdin.next_event().await {
            log::trace!("event: {:?}", event);
            match event {
                Event::Char('\n') => {
                    editor.stdout.write_all(b"\r\n").await.unwrap();

                    if !editor.buffer.text().is_empty() {
                        break;
                    }
                }
                Event::Left | Event::Ctrl('b') => {
                editor.buffer.move_cursor_relative(-1);
                }
                Event::Right | Event::Ctrl('f') => {
                    editor.buffer.move_cursor_relative(1);
                }
                Event::Up => {
                    let history = editor.history.clone();

                    match editor.history_cursor.get_or_insert_with(|| history.entries()).next() {
                        Some(entry) => {
                            // TODO: Save buffer for later if user wants to return to
                            // what they typed.
                            editor.buffer.clear();
                            editor.buffer.insert_str(entry.command());
                        }
                        None => {
                            // TODO
                        }
                    }
                }
                Event::Down => {
                    if let Some(mut cursor) = editor.history_cursor.take() {
                        editor.buffer.clear();

                        if let Some(entry) = cursor.prev() {
                            editor.buffer.insert_str(entry.command());
                            editor.history_cursor = Some(cursor);
                        }
                    }

                    // TODO: Restore original buffer
                }
                Event::Home | Event::Ctrl('a') => {
                    editor.buffer.move_to_start_of_line();
                }
                Event::End | Event::Ctrl('e') => {
                    editor.buffer.move_to_end_of_line();
                }
                Event::Char(c) => {
                    editor.buffer.insert_char(c);
                }
                Event::Backspace => {
                    editor.buffer.delete_before_cursor();
                }
                Event::Delete => {
                    editor.buffer.delete_after_cursor();
                }
                Event::Ctrl('c') => {
                    editor.buffer.clear();
                }
                Event::Ctrl('d') | Event::Eof => {
                    if editor.buffer.is_empty() {
                        return ReadLine::Eof;
                    }
                }
                _ => {}
            }

            editor.redraw().await;
        }

        editor.history_cursor = None;

        // Record line to history.
        editor.history_session.add(editor.buffer.text());

        // Move the command line out of out buffer and return it.
        ReadLine::Input(editor.buffer.take_text())
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
