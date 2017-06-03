use execute;
use functions;
use io::{self, IO};
use parser::Expression;


/// Lookup a builtin function by name.
pub fn lookup(name: &str) -> Option<functions::Builtin> {
    match name {
        "builtin" => Some(builtin),
        "capture" | "$" => Some(capture),
        "car" => Some(car),
        "cd" => Some(cd),
        "cdr" => Some(cdr),
        "command" => Some(command),
        "def" => Some(def),
        "do" => Some(do_builtin),
        "exec" => Some(exec),
        "exit" => Some(exit),
        "help" => Some(help),
        "list" => Some(list),
        "pipe" | "|" => Some(pipe),
        "print" | "echo" => Some(print),
        "pwd" => Some(pwd),
        _ => None,
    }
}

/// Executes a builtin command.
pub fn builtin(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(name_expr) = args.first() {
        if let Some(name) = execute::reduce(name_expr, io).atom() {
            if let Some(builtin) = lookup(name) {
                return builtin(&args[1..], io);
            }
        }
    }

    Expression::Nil
}

/// Execute an expression, capturing its standard output and returning it as a value.
pub fn capture(args: &[Expression], io: &mut IO) -> Expression {
    use std::io::{BufRead, BufReader};
    use std::thread;

    // Set up a new IO context with a piped stdout so we can capture it.
    let (write, read) = io::pipe();
    let mut captured_io = io.clone();
    captured_io.stdout = write;

    // Execute the arguments as an expression in the background.
    let expr = Expression::List(args.to_vec());
    thread::spawn(move || {
        execute::execute(&expr, &mut captured_io);
    });

    // Read the first line of output and return it as an atom.
    let mut reader = BufReader::new(read);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    // Trim trailing newline.
    if line.ends_with('\n') {
        line.pop();
    }

    Expression::Atom(line)
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

/// Change the current directory.
pub fn cd(args: &[Expression], io: &mut IO) -> Expression {
    use std::env;

    if let Some(expr) = args.first() {
        if let Some(path) = execute::reduce(expr, io).atom() {
            env::set_current_dir(path).unwrap();
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

/// Define a new function.
pub fn def(args: &[Expression], io: &mut IO) -> Expression {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // First argument is the function name.
    if let Some(name) = execute::reduce(args.first().unwrap(), io).atom() {
        // Second argument is the body. If body wasn't given, just use Nil as the body.
        let body = args.get(1).cloned().unwrap_or(Expression::Nil);

        // Create the function.
        functions::create(name, body);
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
    use std::os::unix::process::CommandExt;

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
            execute::reduce(&expr, &mut child_io)
        }));
    }

    // Wait for all processes to complete and collect their return values.
    let results: Vec<Expression> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    Expression::List(results)
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

/// Print the current directory.
pub fn pwd(args: &[Expression], io: &mut IO) -> Expression {
    use std::env;
    use std::io::Write;

    writeln!(io.stdout, "{}", env::current_dir().unwrap().display()).unwrap();

    Expression::Nil
}
