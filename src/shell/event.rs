/// Enumeration of possible input events that could be received from the user.
pub enum Event {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Insert,
    Backspace,
    Delete,
    Ctrl(char),
}
