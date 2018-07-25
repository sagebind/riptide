use lua;
use lua::ffi::lua_State;
use std::io::{BufRead, BufReader};
use std::process::*;


struct CommandInfo {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

/// Execute an external command.
pub unsafe extern fn command(state: *mut lua_State) -> i32 {
    let mut state = lua::State::from_ptr(state);

    // Create a command for the given program name.
    let mut command = Command::new(state.check_string(1));

    // For each other parameter given, add it as a shell argument.
    for i in 2..state.get_top()+1 {
        // Expand each argument as we go.
        command.arg(state.check_string(i));
    }

    command.stdout(Stdio::piped());

    // Start running the command process.
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => super::throw(&mut state, format!("failed to execute process: {}", e)),
    };

    // Callback to handle a coroutine resume.
    fn continuation(state: &mut lua::State, mut cmd: CommandInfo) -> i32 {
        let mut buf = String::new();

        match cmd.stdout.read_line(&mut buf) {
            Ok(0) => {
                let status = cmd.child.wait().unwrap();
                state.push(status.code().unwrap_or(0) as f64);
                1
            },
            Ok(_) => {
                // Trim trailing newline before yielding.
                if buf.ends_with('\n') {
                    state.push(&buf[0..buf.len() - 1]);
                } else {
                    state.push(buf);
                }

                state.co_yieldk(1, move |state, _| {
                    continuation(state, cmd)
                });
            },
            Err(_) => {
                0
            },
        }
    }

    let stdout = BufReader::new(child.stdout.take().unwrap());

    continuation(&mut state, CommandInfo {
        child: child,
        stdout: stdout,
    })
}
