use builtins;
use parser::Expression;


/// Execute an expression as a function call.
pub fn execute(expr: &Expression) {
    reduce(expr);
}

/// Reduce the given expression to an atom by execution.
///
/// This is the core of the execution code. Most of a program is executed in terms of lazily reducing everything to an
/// atom.
pub fn reduce(expr: &Expression) -> Expression {
    match expr {
        // Expression is already reduced.
        &Expression::Atom(_) => expr.clone(),
        // A list to be executed.
        &Expression::List(ref atoms) => {
            // First get the first atom. This will be the function to execute.
            let f = match atoms.first() {
                Some(expr) => reduce(expr),
                // List is empty, so return empty.
                None => return Expression::Nil,
            };

            if let Some(name) = f.atom() {
                // Try to execute a builtin first.
                if let Some(func) = builtins::get(name) {
                    func(&atoms[1..]);
                } else {
                    // Execute a command.
                    builtins::command::main(atoms);
                }
            }

            Expression::Nil
        }
        &Expression::Nil => Expression::Nil,
    }
}
