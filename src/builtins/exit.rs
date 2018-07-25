use lua;
use lua::ffi::lua_State;


static mut EXIT_FLAG: bool = false;


/// Get the currently set exit code for the current shell.
#[inline]
pub fn exit_code() -> &'static mut i32 {
    static mut EXIT_CODE: i32 = 0;

    unsafe {
        &mut EXIT_CODE
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
pub unsafe extern fn exit(state: *mut lua_State) -> i32 {
    let mut state = lua::State::from_ptr(state);

    let exit_status = state.to_integerx(1).unwrap_or(0);

    *exit_code() = exit_status as i32;
    EXIT_FLAG = true;

    0
}
