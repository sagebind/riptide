use execute;
use io::IO;
use parser::Expression;
use std::os::unix::process::CommandExt;


/// A builtin function in native code.
///
/// Builtin functions have the special property of receiving their arguments before they are reduced.
pub type Builtin = fn(&[Expression], &mut IO) -> Expression;

/// Lookup a builtin function by name.
pub fn get(name: &str) -> Option<Builtin> {
    match name {
        "builtin" => Some(builtin),
        "car" => Some(car),
        "cdr" => Some(cdr),
        "command" => Some(command),
        "do" => Some(do_builtin),
        "exec" => Some(exec),
        "exit" => Some(exit),
        "help" => Some(help),
        "list" => Some(list),
        "pipe" | "|" => Some(pipe),
        "print" | "echo" => Some(print),
        _ => None,
    }
}

/// Executes a builtin command.
pub fn builtin(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(name_expr) = args.first() {
        if let Some(name) = execute::reduce(name_expr, io).atom() {
            if let Some(builtin) = get(name) {
                return builtin(&args[1..], io);
            }
        }
    }

    Expression::Nil
}

/// Return the first element of a list.
pub fn car(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(&Expression::List(ref items)) = args.first() {
        if let Some(item) = items.first() {
            return execute::reduce(item, io);
        }
    }

    Expression::Nil
}

/// Return the tail of a list.
pub fn cdr(args: &[Expression], _: &mut IO) -> Expression {
    if let Some(&Expression::List(ref items)) = args.first() {
        return Expression::List((&items[1..]).to_vec())
    }

    Expression::Nil
}

/// Executes an external command.
pub fn command(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(mut command) = execute::build_external_command(args, io) {
        // Start running the command in a child process.
        command.status();
    }

    Expression::Nil
}

/// Execute expressions in a sequence and return all results in a list.
pub fn do_builtin(args: &[Expression], io: &mut IO) -> Expression {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    let mut results = Vec::new();

    for expr in args {
        results.push(execute::reduce(expr, io));
    }

    Expression::List(results)
}

/// Replace the current process with a new command.
pub fn exec(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(mut command) = execute::build_external_command(args, io) {
        command.exec();
    }

    Expression::Nil
}

/// Exits the current shell.
pub fn exit(args: &[Expression], _: &mut IO) -> Expression {
    use exit;

    *exit::flag() = true;

    Expression::Nil
}

pub fn help(args: &[Expression], io: &mut IO) -> Expression {
    use std::io::Write;

    writeln!(io.stdout, "<PLACEHOLDER TEXT>").unwrap();

    Expression::Nil
}

/// Returns its arguments as an unevaluated list.
pub fn list(args: &[Expression], _: &mut IO) -> Expression {
    Expression::List(args.to_vec())
}

/// Form a pipeline between a series of calls and execute them in parallel.
pub fn pipe(args: &[Expression], io: &mut IO) -> Expression {
    use std::thread;

    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // If only on argument is given, just execute it normally.
    if args.len() == 1 {
        return execute::reduce(&args[0], io);
    }

    // Multiple arguments are given, so create a series of IO contexts that are chained together.
    let mut contexts = io.clone().pipeline(args.len() as u16);
    let mut handles = Vec::new();

    for arg in args {
        let expr = arg.clone();
        let mut child_io = contexts.remove(0);

        handles.push(thread::spawn(move || {
            execute::execute(&expr, &mut child_io);
        }));
    }

    // Wait for all processes to complete.
    for handle in handles {
        handle.join().unwrap();
    }

    Expression::Nil
}

/// Print the given expressions to standard output. Multiple arguments are separated with a space.
pub fn print(args: &[Expression], io: &mut IO) -> Expression {
    use std::io::Write;

    let mut first = true;

    for arg in args {
        let arg = execute::reduce(arg, io);

        if first {
            write!(io.stdout, "{}", arg).unwrap();
            first = false;
        } else {
            write!(io.stdout, " {}", arg).unwrap();
        }
    }

    writeln!(io.stdout).unwrap();

    Expression::Nil
}
