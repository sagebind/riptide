pub mod command;
pub mod exit;
pub mod pipe;

use lua;
use lua::ffi::lua_State;
use std::env;


/// Load builtin functions into the given Lua state.
pub fn load(state: &mut lua::State) {
    state.push_fn(Some(cd));
    state.set_global("cd");

    state.push_fn(Some(pwd));
    state.set_global("pwd");

    state.push_fn(Some(env));
    state.set_global("env");

    state.push_fn(Some(export));
    state.set_global("export");

    state.push_fn(Some(command::command));
    state.set_global("command");

    state.push_fn(Some(exit::exit));
    state.set_global("exit");
}

/// Sets the current working directory.
unsafe extern fn cd(state: *mut lua_State) -> i32 {
    let mut state = lua::State::from_ptr(state);
    let path = state.check_string(1).to_string();

    if env::set_current_dir(path).is_err() {
        throw(&mut state, "failed to change directory");
    }

    0
}

/// Gets the current working directory.
unsafe extern fn pwd(state: *mut lua_State) -> i32 {
    match env::current_dir() {
        Ok(dir) => {
            let mut state = lua::State::from_ptr(state);
            state.push(dir.to_str().unwrap());
            1
        },
        Err(_) => 0,
    }
}

/// Gets the value of an environment variable.
///
/// # Lua arguments
/// * `key: string` - The variable name.
unsafe extern fn env(state: *mut lua_State) -> i32 {
    let mut state = lua::State::from_ptr(state);
    let key = state.check_string(1).to_string();

    if let Ok(value) = env::var(key) {
        state.push(value);
    } else {
        state.push_nil();
    }

    1
}

/// Exports an environment variable.
///
/// # Lua arguments
/// * `key: string` - The variable name.
/// * `value: string` - The value to set.
unsafe extern fn export(state: *mut lua_State) -> i32 {
    let mut state = lua::State::from_ptr(state);
    let key = state.check_string(1).to_string();
    let value = state.check_string(2).to_string();

    env::set_var(key, value);

    0
}


/// Throw an error.
fn throw<S: AsRef<str>>(state: &mut lua::State, message: S) -> ! {
    state.location(1);
    state.push_string(message.as_ref());
    state.concat(2);
    state.error();
}
