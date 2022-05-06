pub enum Command {
    Clear,
    ClearAfterCursor,
    MoveCursorLeft(usize),
    MoveCursorToAbsolute(usize, usize),
    EnableAlternateBuffer,
    DisableAlternateBuffer,
}
