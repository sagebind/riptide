//! Functions for working with processes.

use crate::prelude::*;
use nix::unistd;
use std::{
    ffi::{CString, OsStr},
    future::Future,
    io::ErrorKind,
    process,
};
use tokio::process::Command;

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
///
/// Cancellation is fully supported. Dropping the returned future will send a
/// signal to the child process to terminate.
pub async fn command(fiber: &mut Fiber, command: impl AsRef<OsStr>, args: &[Value]) -> Result<Value, Exception> {
    // Here we clone the stdin file descriptor, because Command wants to take
    // ownership of it.
    let mut stdin = fiber.stdin().try_clone()?;

    // Ensure we restore non-blocking once finished. Use a scope guard to ensure
    // that the non-blocking flag is restored even on cancellation.
    let mut fiber = scopeguard::guard(fiber, |fiber| {
        if let Err(e) = fiber.stdin().set_nonblocking(true) {
            log::warn!("failed to restore stdin to non-blocking mode: {}", e);
        }
    });

    // Most processes assume standard I/O streams begin life as blocking. Note
    // that this flag affects all file descriptors pointing to the same file
    // description, so we must make sure to restore this when we're done.
    stdin.set_nonblocking(false)?;

    let exit_status = Command::new(command)
        .args(args.iter().map(|value| crate::string::RipString::from(value.clone())))
        .current_dir(fiber.current_dir().to_string())
        .stdin(stdin)
        .stdout(fiber.stdout().try_clone()?)
        .stderr(fiber.stderr().try_clone()?)
        .kill_on_drop(true)
        .status()
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => Exception::from("no such command or file"),
            _ => e.to_string().into(),
        })?;

    if exit_status.success() {
        Ok(Value::Nil)
    } else {
        Err(Exception::from(Value::from(exit_status.code().unwrap_or(0) as f64)))
    }
}

/// Spawn a new child process and execute the given future in it.
///
/// Returns the PID of the child process.
pub async fn spawn<F: Future<Output = ()>>(future: F) -> Result<i32, String> {
    match unistd::fork() {
        Ok(unistd::ForkResult::Child) => {
            // If the given future panics, then ensure that we exit here while
            // unwinding.
            scopeguard::defer_on_unwind! {
                process::exit(1);
            }

            future.await;

            // Success, terminate the process.
            process::exit(0);
        }

        Ok(unistd::ForkResult::Parent {
            child,
        }) => Ok(child.into()),

        Err(e) => Err(e.to_string()),
    }
}

/// Replace the current process with an external command.
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
