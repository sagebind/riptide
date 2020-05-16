/// Enumeration of possible input events that could be received from the user.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    Eof,
}
