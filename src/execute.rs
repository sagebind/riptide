use builtins;
use io::IO;
use parser::Expression;
use std::os::unix::io::*;
use std::process::{Command, Stdio};


/// Execute an expression as a function call.
pub fn execute(expr: &Expression, io: &mut IO) {
    reduce(expr, io);
}

/// Reduce the given expression to an atom by execution.
///
/// This is the core of the execution code. Most of a program is executed in terms of lazily reducing everything to an
/// atom.
pub fn reduce(expr: &Expression, io: &mut IO) -> Expression {
    match expr {
        // Expression is already reduced.
        &Expression::Atom(_) => expr.clone(),
        // A list to be executed.
        &Expression::List(ref atoms) => {
            // First get the first atom. This will be the function to execute.
            let f = match atoms.first() {
                Some(expr) => reduce(expr, io),
                // List is empty, so return empty.
                None => return Expression::Nil,
            };

            if let Some(name) = f.atom() {
                // Try to execute a builtin first.
                if let Some(func) = builtins::get(name) {
                    return func(&atoms[1..], io);
                } else {
                    // Execute a command.
                    return builtins::command(atoms, io);
                }
            }

            Expression::Nil
        }
        &Expression::Nil => Expression::Nil,
    }
}

/// Create a command builder from an expression list.
pub fn build_external_command(args: &[Expression], io: &mut IO) -> Option<Command> {
    // Get the name of the program to execute.
    if let Some(Expression::Atom(name)) = args.first().map(|e| reduce(e, io)) {
        // Create a command for the given program name.
        let mut command = Command::new(name);

        // For each other parameter given, add it as a shell argument.
        for arg in &args[1..] {
            // Reduce each argument as we go.
            command.arg(reduce(arg, io).atom().unwrap());
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

        Some(command)
    } else {
        None
    }
}
