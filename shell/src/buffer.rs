use std::mem;

/// State of a prompt buffer.
pub struct Buffer {
    // Current command line buffer text.
    text: String,
    // Current cursor position in the buffer.
    cursor: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the current buffer text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Take the text buffer out of the prompt.
    pub fn take_text(&mut self) -> String {
        let mut text = String::new();
        mem::swap(&mut self.text, &mut text);

        self.cursor = 0;

        text
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn cursor_is_at_end_of_line(&self) -> bool {
        self.cursor == self.text.len()
    }

    /// Move the cursor to the given position.
    ///
    /// Returns the new cursor position. The actual position may differ if the requested position was beyond the end of
    /// the buffer.
    pub fn move_cursor_to(&mut self, pos: usize) -> usize {
        self.cursor = self.text.len().min(pos);
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
        let pos = self.text.len();
        self.move_cursor_to(pos);
    }

    /// Insert a character after the cursor.
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.move_cursor_relative(1);
    }

    /// Insert a string after the cursor.
    pub fn insert_str<S: AsRef<str>>(&mut self, string: S) {
        let string = string.as_ref();
        self.text.insert_str(self.cursor as usize, string);
        self.move_cursor_relative(string.len() as isize);
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.text.remove(self.cursor - 1);
            self.move_cursor_relative(-1);
        }
    }

    pub fn delete_after_cursor(&mut self) {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    /// Clears the buffer text and moves the cursor to the beginning.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
}
