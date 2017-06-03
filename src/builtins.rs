use functions;
use interpreter;
use interpreter::Expression;
use io::{self, IO};
use parser;


/// Lookup a builtin function by name.
pub fn lookup(name: &str) -> Option<functions::Builtin> {
    match name {
        "and" => Some(builtin_and),
        "begin" => Some(builtin_begin),
        "builtin" => Some(builtin),
        "=" => Some(builtin_equal),
        "capture" | "$" => Some(capture),
        "car" => Some(car),
        "cd" => Some(cd),
        "cdr" => Some(cdr),
        "command" => Some(command),
        "crush" => Some(builtin_crush),
        "def" => Some(def),
        "exec" => Some(exec),
        "exit" => Some(exit),
        "help" => Some(help),
        "if" => Some(builtin_if),
        "list" => Some(list),
        "not" => Some(builtin_not),
        "pipe" | "|" => Some(pipe),
        "print" | "echo" => Some(print),
        "pwd" => Some(pwd),
        "source" => Some(source),
        _ => None,
    }
}

/// Test if all arguments are truthy.
pub fn builtin_and(args: &[Expression], io: &mut IO) -> Expression {
    for arg in args {
        if !interpreter::reduce(arg, io).is_truthy() {
            return Expression::Atom("false".into());
        }
    }

    Expression::Atom("true".into())
}

/// Executes a builtin command.
pub fn builtin(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(name_expr) = args.first() {
        if let Some(name) = interpreter::reduce(name_expr, io).atom() {
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
        interpreter::execute(&expr, &mut captured_io);
    });

    // Read the first line of output and return it as an atom.
    let mut reader = BufReader::new(read);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    // Trim trailing newline.
    if line.ends_with('\n') {
        line.pop();
    }

    Expression::Atom(line.into())
}

/// Return the first element of a list.
pub fn car(args: &[Expression], io: &mut IO) -> Expression {
    if let Some(&Expression::List(ref items)) = args.first() {
        if let Some(item) = items.first() {
            return interpreter::reduce(item, io);
        }
    }

    Expression::Nil
}

/// Change the current directory.
pub fn cd(args: &[Expression], io: &mut IO) -> Expression {
    use std::env;

    if let Some(expr) = args.first() {
        if let Some(path) = interpreter::reduce(expr, io).atom() {
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
    if let Some(mut command) = interpreter::build_external_command(args, io) {
        // Start running the command in a child process.
        let status = command.status().expect("error running external command");

        // Return the exit code.
        let status_string = format!("{}", status.code().unwrap_or(0));
        return Expression::Atom(status_string.into());
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
    if let Some(name) = interpreter::reduce(args.first().unwrap(), io).atom() {
        // Second argument is the body. If body wasn't given, just use Nil as the body.
        let body = args.get(1).cloned().unwrap_or(Expression::Nil);

        // Create the function.
        functions::create(name, body);
    }

    Expression::Nil
}

/// Execute expressions in a sequence and returns the result of the last expression.
pub fn builtin_begin(args: &[Expression], io: &mut IO) -> Expression {
    let mut result = Expression::Nil;

    for expr in args {
        result = interpreter::reduce(expr, io);
    }

    result
}

pub fn builtin_crush(args: &[Expression], io: &mut IO) -> Expression {
    use std::env;

    if let Ok(path) = env::current_exe() {
        let mut args = args.to_vec();
        args.insert(0, Expression::Atom(path.to_string_lossy().into_owned().into()));
        command(&args, io)
    } else {
        Expression::Nil
    }
}

/// Test if all arguments are equal.
pub fn builtin_equal(args: &[Expression], io: &mut IO) -> Expression {
    // If less than two arguments are given, just return true.
    if args.len() < 2 {
        return Expression::True();
    }

    let expr_to_compare_to = &args[0];
    for expr in &args[1..] {
        if expr != expr_to_compare_to {
            return Expression::False();
        }
    }

    return Expression::True();
}

/// Replace the current process with a new command.
pub fn exec(args: &[Expression], io: &mut IO) -> Expression {
    use std::os::unix::process::CommandExt;

    if let Some(mut command) = interpreter::build_external_command(args, io) {
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

/// If the first argument is truthy, the second argument is executed. Otherwise the third argument is executed.
pub fn builtin_if(args: &[Expression], io: &mut IO) -> Expression {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // Determine if the first argument is truthy.
    let condition_expr = interpreter::reduce(&args[0], io);
    let truthy = condition_expr.is_truthy();

    // Evaluate the appropriate expression arm.
    if truthy {
        if let Some(expr) = args.get(1) {
            interpreter::reduce(expr, io)
        } else {
            Expression::Nil
        }
    } else {
        if let Some(expr) = args.get(2) {
            interpreter::reduce(expr, io)
        } else {
            Expression::Nil
        }
    }
}

/// Returns its arguments as an unevaluated list.
pub fn list(args: &[Expression], io: &mut IO) -> Expression {
    interpreter::execute_multiple(args, io)
}

pub fn builtin_not(args: &[Expression], _: &mut IO) -> Expression {
    Expression::Nil
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
        return interpreter::reduce(&args[0], io);
    }

    // Multiple arguments are given, so create a series of IO contexts that are chained together.
    let mut contexts = io.clone().pipeline(args.len() as u16);
    let mut handles = Vec::new();

    for arg in args {
        let expr = arg.clone();
        let mut child_io = contexts.remove(0);

        handles.push(thread::spawn(move || {
            interpreter::reduce(&expr, &mut child_io)
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
        let arg = interpreter::reduce(arg, io);

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

/// Evaluate the contents of a file and return its result.
pub fn source(args: &[Expression], io: &mut IO) -> Expression {
    use std::fs::File;

    // If a filename is given, read from the file. Otherwise read from stdin.
    let expr = if args.is_empty() {
        parser::parse_stream(&mut io.stdin)
    } else if let Some(filename) = interpreter::reduce(&args[0], io).atom() {
        let mut file = File::open(filename).unwrap();
        parser::parse_stream(&mut file)
    } else {
        return Expression::Nil;
    };

    if let Ok(expr) = expr {
        interpreter::reduce(&expr, io)
    } else {
        Expression::Nil
    }
}
