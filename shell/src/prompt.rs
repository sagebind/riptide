use std::mem;

/// State of a prompt buffer.
pub struct Prompt {
    // Current command line buffer.
    buffer: String,
    // Current cursor position in the buffer.
    cursor: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    /// Get the current buffer text.
    pub fn text(&self) -> &str {
        &self.buffer
    }

    /// Take the text buffer out of the prompt.
    pub fn take_text(&mut self) -> String {
        let mut buffer = String::new();
        mem::swap(&mut self.buffer, &mut buffer);

        self.cursor = 0;

        buffer
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move the cursor to the given position.
    ///
    /// Returns the new cursor position. The actual position may differ if the requested position was beyond the end of
    /// the buffer.
    pub fn move_cursor_to(&mut self, pos: usize) -> usize {
        self.cursor = self.buffer.len().min(pos);
        self.cursor
    }

    /// Adjust the cursor position by a relative amount.
    ///
    /// Returns the new cursor position.
    pub fn move_cursor_relative(&mut self, offset: isize) -> usize {
        let pos = 0.max(self.cursor as isize + offset) as usize;
        self.move_cursor_to(pos)
    }

    pub fn move_to_start_of_line(&mut self) {
        self.move_cursor_to(0);
    }

    pub fn move_to_end_of_line(&mut self) {
        let pos = self.buffer.len();
        self.move_cursor_to(pos);
    }

    /// Insert a character after the cursor.
    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.move_cursor_relative(1);
    }

    /// Insert a string after the cursor.
    pub fn insert_str<S: AsRef<str>>(&mut self, string: S) {
        let string = string.as_ref();
        self.buffer.insert_str(self.cursor as usize, string);
        self.move_cursor_relative(string.len() as isize);
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.buffer.remove(self.cursor - 1);
            self.move_cursor_relative(-1);
        }
    }

    pub fn delete_after_cursor(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// Clears the buffer text and moves the cursor to the beginning.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }
}
