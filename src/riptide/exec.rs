//! Functions for executing external programs.
use expr::Expression;
use io::Streams;
use std::os::unix::io::*;
use std::process::{Command, Stdio};


/// Create a command builder from an expression list.
pub fn build_external_command(name: &str, args: &[Expression], streams: &mut Streams) -> Command {
    // Create a command for the given program name.
    let mut command = Command::new(name);

    // For each other parameter given, add it as a shell argument.
    for arg in args {
        // Reduce each argument as we go.
        command.arg(arg.as_value().unwrap());
    }

    // Set up standard IO streams.
    command.stdin(unsafe {
        Stdio::from_raw_fd(streams.stdin.clone().into_raw_fd())
    });
    command.stdout(unsafe {
        Stdio::from_raw_fd(streams.stdout.clone().into_raw_fd())
    });
    command.stderr(unsafe {
        Stdio::from_raw_fd(streams.stderr.clone().into_raw_fd())
    });

    command
}
