use crate::io::process;
use crate::prelude::*;
use std::thread;
use std::time::Duration;
use tokio::process::Command;

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "command" => Value::foreign_fn(command),
        "exec" => Value::foreign_fn(exec),
        "sleep" => Value::foreign_fn(sleep),
        "spawn" => Value::foreign_fn(spawn),
    }
    .into())
}

/// Spawns a new child process and executes a given block in it.
///
/// Returns the child process PID.
async fn spawn(_: &mut Fiber, _: &[Value]) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        // let child_interpreter = Runtime::new();
        // child_interpreter.execute(Exp)
    })
    .unwrap();

    Ok(Value::Number(pid as f64))
}

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
async fn command(fiber: &mut Fiber, args: &[Value]) -> Result<Value, Exception> {
    if let Some(command) = args.first() {
        let command = crate::string::RipString::from(command.clone());

        crate::io::process::command(fiber, command, &args[1..]).await
    } else {
        throw!("command to execute is required")
    }
}

/// Executes a shell command, replacing the current process with the new process.
///
/// Does not return.
async fn exec(_: &mut Fiber, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Puts the current process to sleep for a given number of seconds.
async fn sleep(_: &mut Fiber, args: &[Value]) -> Result<Value, Exception> {
    if let Some(Value::Number(seconds)) = args.first() {
        let seconds = *seconds;

        let duration = if seconds.is_normal() && seconds > 0f64 {
            Duration::new(
                seconds.trunc() as u64,
                (seconds.fract() * 1_000_000_000f64) as u32,
            )
        } else {
            Duration::from_secs(0)
        };

        log::debug!("sleeping for {}ms", duration.as_millis());
        thread::sleep(duration);

        Ok(Value::Nil)
    } else {
        throw!("sleep duration required")
    }
}
