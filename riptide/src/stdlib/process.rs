use prelude::*;
use process;
use std::process::Command;

/// Spawns a new child process and executes a given block in it.
///
/// Returns the child process PID.
pub fn spawn(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        // let child_interpreter = Runtime::new();
        // child_interpreter.execute(Exp)
    }).unwrap();

    Ok(Value::Number(pid as f64))
}

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
pub fn command(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if let Some(command) = args.first() {
        let command = command
            .as_string()
            .and_then(|s| s.as_utf8())
            .ok_or_else(|| Exception::from("invalid command name"))?;

        let mut string_args = Vec::new();

        if args.len() > 1 {
            for arg in &args[1..] {
                string_args.push(arg
                    .as_string()
                    .and_then(|s| s.as_utf8())
                    .ok_or_else(|| Exception::from("argument must be UTF-8"))?);
            }
        }

        Command::new(command)
            .args(string_args)
            .status()
            .map(|status| Value::from(status.code().unwrap_or(0) as f64))
            .map_err(|e| e.to_string().into())


    } else {
        Err(Exception::from("command to execute is required"))
    }
}

/// Executes a shell command, replacing the current process with the new process.
///
/// Does not return.
pub fn exec(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}
