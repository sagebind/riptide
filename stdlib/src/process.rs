use riptide_runtime::io::process;
use riptide_runtime::{
    prelude::*,
    table,
    throw,
};
use std::time::Duration;

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "command" => Value::foreign_fn(command),
        "exec" => Value::foreign_fn(exec),
        "pid" => Value::foreign_fn(pid),
        "sleep" => Value::foreign_fn(sleep),
        "spawn" => Value::foreign_fn(spawn),
    }
    .into())
}

/// Spawns a new child process and executes a given block in it.
///
/// Returns the child process PID.
async fn spawn(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let block = match args.first() {
        Some(arg) if arg.type_name() == "block" => arg.clone(),
        _ => throw!("a block to execute must be provided"),
    };
    let child_args = args.iter().cloned().skip(1).collect::<Vec<_>>();

    // Create a child fiber to correspond to the child process, otherwise the
    // child will try and share file descriptors with the parent.
    // TODO: This is borken somehow, as the child process is still messing with
    // the parent's file descriptors somewhere resulting in an EBADF error.
    let mut child_fiber = fiber.fork();

    let pid = process::spawn(async {
        child_fiber.invoke(&block, &child_args).await.unwrap();
    }).await?;

    Ok(Value::Number(pid as f64))
}

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
async fn command(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    if let Some(Value::String(command)) = args.first() {
        process::command(fiber, command, &args[1..]).await
    } else {
        throw!("command to execute is required")
    }
}

/// Executes a shell command, replacing the current process with the new process.
///
/// Does not return.
async fn exec(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    unimplemented!();
}

/// Puts the current process to sleep for a given number of seconds.
async fn sleep(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
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
        tokio::time::sleep(duration).await;

        Ok(Value::Nil)
    } else {
        throw!("sleep duration required")
    }
}

/// Get the current process' ID.
async fn pid(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    Ok(std::process::id().into())
}
