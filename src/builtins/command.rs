use execute;
use parser::Expression;
use std::process::*;


/// Executes an external command.
pub fn main(args: &[Expression]) {
    // Get the name of the program to execute.
    if let Some(Expression::Atom(name)) = args.first().map(execute::reduce) {
        // Create a command for the given program name.
        let mut command = Command::new(name);

        // For each other parameter given, add it as a shell argument.
        for arg in &args[1..] {
            // Reduce each argument as we go.
            command.arg(execute::reduce(arg).atom().unwrap());
        }

        // Start running the command process.
        command.status();
    }
}
