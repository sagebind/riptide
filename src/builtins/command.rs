use execute;
use parser::Expression;
use std::process::*;


/// Create a command builder from an expression list.
pub fn build_command(args: &[Expression]) -> Option<Command> {
    // Get the name of the program to execute.
    if let Some(Expression::Atom(name)) = args.first().map(execute::reduce) {
        // Create a command for the given program name.
        let mut command = Command::new(name);

        // For each other parameter given, add it as a shell argument.
        for arg in &args[1..] {
            // Reduce each argument as we go.
            command.arg(execute::reduce(arg).atom().unwrap());
        }

        Some(command)
    } else {
        None
    }
}

/// Executes an external command.
pub fn main(args: &[Expression]) {
    if let Some(mut command) = build_command(args) {
        // Start running the command in a child process.
        command.status();
    }
}
