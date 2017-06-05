//! Functions for executing external programs.
use interpreter;
use io::IO;
use parser::Expression;
use std::os::unix::io::*;
use std::process::{Command, Stdio};


/// Create a command builder from an expression list.
pub fn build_external_command(args: &[Expression], io: &mut IO) -> Option<Command> {
    // Get the name of the program to execute.
    if let Some(expr) = args.first().map(|e| interpreter::execute(e, io)) {
        if let Some(name) = expr.value() {
            // Create a command for the given program name.
            let mut command = Command::new(name);

            // For each other parameter given, add it as a shell argument.
            for arg in &args[1..] {
                // Reduce each argument as we go.
                command.arg(interpreter::execute(arg, io).value().unwrap());
            }

            // Set up standard IO streams.
            command.stdin(unsafe {
                Stdio::from_raw_fd(io.stdin.clone().into_raw_fd())
            });
            command.stdout(unsafe {
                Stdio::from_raw_fd(io.stdout.clone().into_raw_fd())
            });
            command.stderr(unsafe {
                Stdio::from_raw_fd(io.stderr.clone().into_raw_fd())
            });

            return Some(command);
        }
    }

    None
}
