//! The Editor is the user interface component of the Riptide shell.

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
mod history_browser;
pub mod prompt;
pub mod scene;

/// Controls the interactive command line editor.
pub struct Editor<I, O: AsRawFd, C> {
    stdin: TerminalInput<I>,
    stdout: TerminalOutput<O>,
    history: History,
    history_session: Session,
    history_cursor: Option<EntryCursor>,
    completer: C,
    buffer: Buffer,

    // A stack of scenes. The topmost scene is the active one and receives all
    // user input.
    scene_stack: Vec<Box<dyn scene::Scene>>,
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
            scene_stack: Vec::new(),
        }
    }

    // TODO: Determine how this is configured.
    fn get_theme(&self) -> Theme {
        Theme::default()
    }

    fn push_scene<S: scene::Scene + 'static>(&mut self, scene: S) {
        self.scene_stack.push(Box::new(scene));
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

        // Enter raw mode.
        let _raw_guard = self.stdout.enter_raw_mode();

        // Handle keyboard events.
        while let Ok(event) = self.stdin.next_event().await {
            log::trace!("event: {:?}", event);

            match self.on_event(event).await {
                Some(ReadLine::Input(text)) => {
                    self.history_cursor = None;

                    // Record line to history.
                    self.history_session.add(&text);

                    return ReadLine::Input(text);
                }
                Some(r) => return r,
                None => {}
            }

            self.render(fiber).await;
        }

        panic!()
    }

    async fn on_event(&mut self, event: Event) -> Option<ReadLine> {
        let mut ctx = scene::SceneContext::default();
        let mut handled = false;

        if let Some(scene) = self.scene_stack.last_mut() {
            scene.on_input(&mut ctx, event);
            handled = true;
        }

        if ctx.close {
            self.scene_stack.pop();
        }

        if handled {
            return None;
        }

        match event {
            Event::Char('\n') => {
                self.stdout.write_all(b"\r\n").await.unwrap();

                if !self.buffer.text().is_empty() {
                    // Move the command line out of out buffer and return it.
                    return Some(ReadLine::Input(self.buffer.take_text()));
                }
            }
            Event::Left | Event::Ctrl('b') => {
                self.buffer.move_cursor_relative(-1);
            }
            Event::Right | Event::Ctrl('f') => {
                if self.buffer.cursor_is_at_end_of_line() {
                    // If the cursor is already at the end of the line, then
                    // fill in the current suggested command, if any.
                    // TODO: Only compute suggestion one time each event.
                    if let Some(suggestion) = self.completer.complete_one(self.buffer.text()) {
                        if let Some(suffix) = suggestion.strip_prefix(self.buffer.text()) {
                            if !suffix.is_empty() {
                                self.buffer.insert_str(suffix);
                            }
                        }
                    }
                } else {
                    // Advance the cursor right as normal.
                    self.buffer.move_cursor_relative(1);
                }
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
            Event::Home | Event::Ctrl('a') => {
                self.buffer.move_to_start_of_line();
            }
            Event::End | Event::Ctrl('e') => {
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
            Event::Ctrl('d') | Event::Eof => {
                if self.buffer.is_empty() {
                    return Some(ReadLine::Eof);
                }
            }
            Event::Ctrl('h') => {
                self.push_scene(history_browser::HistoryBrowser::default());
            }
            _ => {}
        }

        None
    }

    /// Redraw the buffer.
    pub async fn render(&mut self, fiber: &mut Fiber) {
        if let Some(scene) = self.scene_stack.last() {
            // TODO: Alt buffer should depend on the scene.
            self.stdout
                .command(Command::EnableAlternateBuffer)
                .await
                .unwrap();
            self.stdout
                .command(Command::Clear)
                .await
                .unwrap();
            self.stdout
                .command(Command::MoveCursorToAbsolute(1, 1))
                .await
                .unwrap();

            let buffer = scene.render();
            self.stdout.write_all(buffer.as_bytes()).await.unwrap();

            return;
        } else {
            self.stdout
                .command(Command::DisableAlternateBuffer)
                .await
                .unwrap();
        }

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
