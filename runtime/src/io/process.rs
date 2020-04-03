//! Functions for working with processes.

use crate::prelude::*;
use nix::unistd;
use std::{
    ffi::{CString, OsStr},
    future::Future,
    process,
};
use tokio::process::Command;

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
pub async fn command(fiber: &mut Fiber, command: impl AsRef<OsStr>, args: &[Value]) -> Result<Value, Exception> {
    let mut stdin = fiber.stdin().try_clone()?;

    // Most processes assume standard I/O streams begin life as blocking.
    stdin.set_nonblocking(false)?;

    let result = Command::new(command)
        .args(args.iter().map(|value| crate::string::RipString::from(value.clone())))
        .stdin(stdin)
        .stdout(fiber.stdout().try_clone()?)
        .stderr(fiber.stderr().try_clone()?)
        .status()
        .await
        .map(|status| Value::from(status.code().unwrap_or(0) as f64))
        .map_err(|e| e.to_string().into());

    // Restore non-blocking.
    fiber.stdin().set_nonblocking(true)?;

    result
}

/// Spawn a new child process and execute the given future in it.
///
/// Returns the PID of the child process.
pub async fn spawn<F: Future<Output = ()>>(future: F) -> Result<i32, ()> {
    match unistd::fork() {
        Ok(unistd::ForkResult::Child) => {
            future.await;
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
