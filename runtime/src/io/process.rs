//! Functions for working with processes.
use nix::unistd;
use std::ffi::CString;
use std::process;

pub fn command() {}

/// Spawn a new child process and execute the given function in it.
///
/// Returns the PID of the child process.
pub fn spawn<F: FnOnce()>(body: F) -> Result<i32, ()> {
    match unistd::fork() {
        Ok(unistd::ForkResult::Child) => {
            (body)();
            process::exit(0);
        }

        Ok(unistd::ForkResult::Parent {
            child,
        }) => Ok(child.into()),

        Err(_) => Err(()),
    }
}

pub fn exec(command: &str, args: &[&str]) -> Result<(), String> {
    let command_c = CString::new(command).unwrap();

    let mut args_c = Vec::new();
    for arg in args {
        args_c.push(CString::new(*arg).unwrap());
    }

    match unistd::execvp(&command_c, &args_c.iter().map(|s| s.as_c_str()).collect::<Vec<_>>()) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
