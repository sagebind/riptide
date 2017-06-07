use exec;
use interpreter::{self, Context};
use io::{self, IO};
use parser::{self, Expression};


/// A builtin function in native code.
///
/// Builtin functions have the special property of receiving their arguments before they are reduced.
pub type Builtin = fn(&[Expression], &Context, &mut IO) -> Expression;

/// Lookup a builtin function by name.
pub fn lookup(name: &str) -> Option<Builtin> {
    match name {
        "and" => Some(builtin_and),
        "args" => Some(builtin_args),
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
        "env" => Some(builtin_env),
        "exec" => Some(exec),
        "exit" => Some(exit),
        "help" => Some(help),
        "if" => Some(builtin_if),
        "list" => Some(list),
        "not" => Some(builtin_not),
        "pipe" | "|" => Some(pipe),
        "print" | "echo" => Some(print),
        "pwd" => Some(pwd),
        "quote" => Some(quote),
        "source" => Some(source),
        _ => None,
    }
}

/// Test if all arguments are truthy.
pub fn builtin_and(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    for arg in args {
        if !interpreter::execute_function_call(arg, context, io).is_truthy() {
            return Expression::Atom("false".into());
        }
    }

    Expression::Atom("true".into())
}

/// Return all arguments passed to the current function as a list.
pub fn builtin_args(_: &[Expression], context: &Context, io: &mut IO) -> Expression {
    interpreter::execute_all(&*context.args, context, io)
}

/// Executes a builtin command.
pub fn builtin(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    if let Some(name_expr) = args.first() {
        if let Some(name) = interpreter::execute_function_call(name_expr, context, io).value() {
            if let Some(builtin) = lookup(name) {
                return builtin(&args[1..], context, io);
            }
        }
    }

    Expression::Nil
}

/// Execute an expression, capturing its standard output and returning it as a value.
pub fn capture(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::io::{BufRead, BufReader};
    use std::thread;

    // Set up a new IO context with a piped stdout so we can capture it.
    let (write, read) = io::pipe();
    let mut captured_io = io.clone();
    captured_io.stdout = write;

    // Execute the arguments as an expression in the background.
    let expr = Expression::List(args.to_vec());
    let context = context.clone();
    thread::spawn(move || {
        interpreter::execute_function_call(&expr, &context, &mut captured_io);
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
pub fn car(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    if let Some(expr) = args.first() {
        let expr = interpreter::execute_function_call(expr, context, io);

        if let Some(items) = expr.items() {
            if let Some(item) = items.first() {
                return item.clone();
            }
        }
    }

    Expression::Nil
}

/// Change the current directory.
pub fn cd(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::env;

    if let Some(expr) = args.first() {
        if let Some(path) = interpreter::execute_function_call(expr, context, io).value() {
            env::set_current_dir(path).unwrap();
        }
    }

    Expression::Nil
}

/// Return the tail of a list.
pub fn cdr(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    if let Some(expr) = args.first() {
        let expr = interpreter::execute_function_call(expr, context, io);

        if let Some(items) = expr.items() {
            return interpreter::execute_all(&items[1..], context, io);
        }
    }

    Expression::Nil
}

/// Executes an external command.
pub fn command(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    if let Some(mut command) = exec::build_external_command(args, io) {
        // Start running the command in a child process.
        let status = command.status().expect("error running external command");

        // Return the exit code.
        let status_string = format!("{}", status.code().unwrap_or(0));
        return Expression::atom(status_string);
    }

    Expression::Nil
}

/// Define a new function.
pub fn def(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // First argument is the function name.
    if let Some(name) = interpreter::execute_function_call(args.first().unwrap(), context, io).value() {
        let mut params = Vec::new();
        let mut body = Expression::Nil;

        // If two arguments are given, there are no parameters and the second argument is the body.
        if args.len() == 2 {
            body = args[1].clone();
        }

        // If three arguments are given, the second argument is the parameter list and the third is the body.
        else if args.len() >= 3 {
            body = args[2].clone();

            if let Some(params_list) = args[1].items() {
                for param_expr in params_list {
                    if let Some(param_name) = interpreter::execute_function_call(param_expr, context, io).value() {
                        params.push(param_name.to_owned());
                    }
                }
            }
        }

        // Create the function.
        interpreter::create_function(name, params, body);
    }

    Expression::Nil
}

/// Execute expressions in a sequence and returns the result of the last expression.
pub fn builtin_begin(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    let mut result = Expression::Nil;

    for expr in args {
        result = interpreter::execute_function_call(expr, context, io);
    }

    result
}

pub fn builtin_crush(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::env;

    if let Ok(path) = env::current_exe() {
        let mut args = args.to_vec();
        args.insert(0, Expression::Atom(path.to_string_lossy().into_owned().into()));
        command(&args, context, io)
    } else {
        Expression::Nil
    }
}

/// Test if all arguments are equal.
pub fn builtin_equal(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    // If less than two arguments are given, just return true.
    if args.len() < 2 {
        return Expression::TRUE;
    }

    let expr_to_compare_to = &args[0];
    for expr in &args[1..] {
        if expr != expr_to_compare_to {
            return Expression::FALSE;
        }
    }

    return Expression::TRUE;
}

pub fn builtin_env(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::env;
    use std::io::Write;

    // If no arguments are given, print all variables.
    if args.is_empty() {
        for (name, value) in env::vars() {
            writeln!(io.stdout, "{} {}", name, value);
        }
    }

    // If one argument is given, lookup and return the value of an environment variable with that name.
    if args.len() == 1 {
        if let Some(name) = interpreter::execute_function_call(&args[0], context, io).value() {
            if let Ok(value) = env::var(name) {
                return Expression::Atom(value.into());
            }
        }
    }

    Expression::Nil
}

/// Replace the current process with a new command.
pub fn exec(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::os::unix::process::CommandExt;

    if let Some(mut command) = exec::build_external_command(args, io) {
        command.exec();
    }

    Expression::Nil
}

/// Exits the current shell.
pub fn exit(args: &[Expression], context: &Context, _: &mut IO) -> Expression {
    use exit;

    *exit::flag() = true;

    Expression::Nil
}

pub fn help(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::io::Write;

    writeln!(io.stdout, "<PLACEHOLDER TEXT>").unwrap();

    Expression::Nil
}

/// If the first argument is truthy, the second argument is executed. Otherwise the third argument is executed.
pub fn builtin_if(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // Determine if the first argument is truthy.
    let condition_expr = interpreter::execute_function_call(&args[0], context, io);
    let truthy = condition_expr.is_truthy();

    // Evaluate the appropriate expression arm.
    if truthy {
        if let Some(expr) = args.get(1) {
            interpreter::execute_function_call(expr, context, io)
        } else {
            Expression::Nil
        }
    } else {
        if let Some(expr) = args.get(2) {
            interpreter::execute_function_call(expr, context, io)
        } else {
            Expression::Nil
        }
    }
}

/// Returns its arguments as a list.
pub fn list(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    interpreter::execute_all(args, context, io)
}

/// Returns its arguments as a list unevaluated.
pub fn quote(args: &[Expression], _: &Context, _: &mut IO) -> Expression {
    Expression::List(args.to_vec())
}

pub fn builtin_not(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    if let Some(expr) = args.first() {
        if interpreter::execute_function_call(expr, context, io).is_truthy() {
            Expression::FALSE
        } else {
            Expression::TRUE
        }
    } else {
        Expression::Nil
    }
}

/// Form a pipeline between a series of calls and execute them in parallel.
pub fn pipe(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::thread;

    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // If only on argument is given, just execute it normally.
    if args.len() == 1 {
        return interpreter::execute_function_call(&args[0], context, io);
    }

    // Multiple arguments are given, so create a series of IO contexts that are chained together.
    let mut contexts = io.clone().pipeline(args.len() as u16);
    let mut handles = Vec::new();

    for arg in args {
        let expr = arg.clone();
        let mut child_io = contexts.remove(0);
        let child_context = context.clone();

        handles.push(thread::spawn(move || {
            interpreter::execute_function_call(&expr, &child_context, &mut child_io)
        }));
    }

    // Wait for all processes to complete and collect their return values.
    let results: Vec<Expression> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    Expression::List(results)
}

/// Print the given expressions to standard output. Multiple arguments are separated with a space.
pub fn print(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::io::Write;

    let mut first = true;

    for arg in args {
        let arg = interpreter::execute_function_call(arg, context, io);

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
pub fn pwd(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::env;
    use std::io::Write;

    writeln!(io.stdout, "{}", env::current_dir().unwrap().display()).unwrap();

    Expression::Nil
}

/// Evaluate the contents of a file and return its result.
pub fn source(args: &[Expression], context: &Context, io: &mut IO) -> Expression {
    use std::fs::File;

    // If a filename is given, read from the file. Otherwise read from stdin.
    let expr = if args.is_empty() {
        parser::parse_stream(&mut io.stdin)
    } else if let Some(filename) = interpreter::execute_function_call(&args[0], context, io).value() {
        let mut file = File::open(filename).unwrap();
        parser::parse_stream(&mut file)
    } else {
        return Expression::Nil;
    };

    if let Ok(expr) = expr {
        interpreter::execute_function_call(&expr, context, io)
    } else {
        Expression::Nil
    }
}
