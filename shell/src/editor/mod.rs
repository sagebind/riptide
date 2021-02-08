use crate::{
    buffer::Buffer,
    completion::Completer,
    editor::{command::Command, event::Event},
    history::{EntryCursor, History, Session},
    os::{TerminalInput, TerminalOutput},
    theme::Theme,
};
use riptide_runtime::{Fiber, Value};
use std::{
    fmt::Write,
    os::unix::io::AsRawFd,
};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use yansi::Paint;

pub mod command;
pub mod event;
pub mod prompt;

/// Controls the interactive command line editor.
pub struct Editor<I, O: AsRawFd, C> {
    stdin: TerminalInput<I>,
    stdout: TerminalOutput<O>,
    history: History,
    history_session: Session,
    history_cursor: Option<EntryCursor>,
    completer: C,
    buffer: Buffer,
}

pub enum ReadLine {
    Input(String),
    Eof,
}

impl<I, O: AsRawFd, C> Editor<I, O, C> {
    pub fn new(stdin: I, stdout: O, history: History, session: Session, completer: C) -> Self {
        Self {
            stdin: TerminalInput::new(stdin),
            stdout: TerminalOutput::new(stdout).unwrap(),
            history,
            history_session: session,
            history_cursor: None,
            completer,
            buffer: Buffer::new(),
        }
    }

    // TODO: Determine how this is configured.
    fn get_theme(&self) -> Theme {
        Theme::default()
    }

    async fn get_prompt_str(&self, fiber: &mut Fiber) -> String {
        match fiber.globals().get("riptide-prompt") {
            // Static prompt.
            Value::String(ref s) => return s.to_string(),

            // Prompt is determined by a callback function.
            value @ Value::Block(_) => match fiber.invoke(&value, &[]).await {
                // Closure returned successfully.
                Ok(Value::String(ref s)) => return s.to_string(),

                // Closure succeeded, but returned an invalid data type.
                Ok(value) => {
                    log::warn!("prompt function returned invalid data type: {}", value.type_name());
                }

                Err(e) => {
                    log::warn!("prompt function threw exception: {}", e);
                }
            },

            Value::Nil => {
                // Unspecified
            }

            value => {
                // Invalid data type
                log::warn!("prompt must be a closure or string, not '{}'", value.type_name());
            }
        }

        let theme = self.get_theme();
        let mut buf = String::new();

        let cwd = fiber.current_dir().to_string();
        write!(
            &mut buf,
            "{}{}",
            Paint::blue(theme.prompt.as_ref().unwrap().item_format.as_ref().unwrap().replace("%s", &cwd)),
            theme.prompt.as_ref().unwrap().item_separator.as_ref().unwrap(),
        ).unwrap();

        write!(
            &mut buf,
            "{} ",
            theme.prompt.as_ref().unwrap().format.as_ref().unwrap(),
        ).unwrap();

        buf
    }
}

impl<I: AsyncRead + Unpin, O: AsyncWrite + AsRawFd + Unpin, C: Completer> Editor<I, O, C> {
    /// Show a command prompt to the user and await for the user to input a
    /// command. The typed command is returned once submitted.
    pub async fn read_line(&mut self, fiber: &mut Fiber) -> ReadLine {
        let prompt = self.get_prompt_str(fiber).await;

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

            editor.redraw(fiber).await;
        }

        editor.history_cursor = None;

        // Record line to history.
        editor.history_session.add(editor.buffer.text());

        // Move the command line out of out buffer and return it.
        ReadLine::Input(editor.buffer.take_text())
    }

    /// Redraw the buffer.
    pub async fn redraw(&mut self, fiber: &mut Fiber) {
        let prompt = self.get_prompt_str(fiber).await;

        // Render the current buffer text.
        self.stdout.write_all(b"\r").await.unwrap();
        self.stdout
            .command(Command::ClearAfterCursor)
            .await
            .unwrap();
        self.stdout
            .write_all(format!("{}{}", prompt, self.buffer.text()).as_bytes())
            .await
            .unwrap();

        // Render the top completion suggestion.
        if !self.buffer.is_empty() {
            if let Some(suggestion) = self.completer.complete_one(self.buffer.text()) {
                if let Some(suffix) = suggestion.strip_prefix(self.buffer.text()) {
                    if !suffix.is_empty() {
                        self.stdout
                            .write_all(format!("{}", Paint::new(suffix).dimmed()).as_bytes())
                            .await
                            .unwrap();

                        self.stdout
                            .command(Command::MoveCursorLeft(suffix.len()))
                            .await
                            .unwrap();
                    }
                }
            }
        }

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
