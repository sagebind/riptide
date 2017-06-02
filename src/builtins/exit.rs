static mut EXIT_FLAG: bool = false;
static mut EXIT_CODE: i32 = 0;


/// Get the currently set exit code for the current shell.
#[inline]
pub fn exit_code() -> i32 {
    unsafe {
        EXIT_CODE
    }
}

/// Check if an exit was triggered by a script.
#[inline]
pub fn exit_requested() -> bool {
    unsafe {
        EXIT_FLAG
    }
}

/// Exit the current shell, optionally with an exit code.
pub fn exit(status: Option<i32>) {
    unsafe {
        EXIT_CODE = status.unwrap_or(0);
        EXIT_FLAG = true;
    }
}
