//! Definition of builtin functions.
use interpreter::{self, NativeFunction, StackFrame};
use io::{self, Streams};
use parser::{self, Expression};


/// Lookup a builtin function by name.
pub fn lookup(name: &str) -> Option<NativeFunction> {
    match name {
        "=" => Some(EQUAL),
        "and" => Some(AND),
        "args" => Some(ARGS),
        "begin" => Some(BEGIN),
        "builtin" => Some(BUILTIN),
        "capture" | "$" => Some(CAPTURE),
        "cd" => Some(CD),
        "command" => Some(COMMAND),
        "crush" => Some(CRUSH),
        "def" => Some(DEF),
        "env" => Some(ENV),
        "exec" => Some(EXEC),
        "exit" => Some(EXIT),
        "first" => Some(FIRST),
        "help" => Some(HELP),
        "if" => Some(IF),
        "list" => Some(LIST),
        "not" => Some(NOT),
        "nth" => Some(NTH),
        "pipe" | "|" => Some(PIPE),
        "print" | "echo" => Some(PRINT),
        "pwd" => Some(PWD),
        "quote" => Some(QUOTE),
        "source" => Some(SOURCE),
        "tail" => Some(TAIL),
        _ => None,
    }
}


/// Convenience macro for defining builtins.
macro_rules! builtin {
    ($args:pat, $frame:pat, $streams:pat, $body:expr) => ({
        fn builtin($args: &[Expression], $frame: &mut StackFrame, $streams: &mut Streams) -> Expression {
            $body
        }

        $crate::interpreter::NativeFunction {
            lazy_args: false,
            ptr: builtin,
        }
    });

    (lazy $args:pat, $frame:pat, $streams:pat, $body:expr) => ({
        fn builtin($args: &[Expression], $frame: &mut StackFrame, $streams: &mut Streams) -> Expression {
            $body
        }

        $crate::interpreter::NativeFunction {
            lazy_args: true,
            ptr: builtin,
        }
    });
}


/// Tests if all arguments are truthy.
pub const AND: NativeFunction = builtin!(args, _, _, {
    for arg in args {
        if !arg.is_truthy() {
            return Expression::FALSE;
        }
    }

    Expression::TRUE
});

/// Return all arguments passed to the current function as a list.
pub const ARGS: NativeFunction = builtin!(_, frame, _, {
    let args = frame.args.to_vec();

    Expression::List(args)
});

/// Execute expressions in a sequence and returns the result of the last expression.
pub const BEGIN: NativeFunction = builtin!(lazy args, frame, streams, {
    for (i, arg) in args.iter().enumerate() {
        let result = interpreter::execute(arg, frame, streams);

        if i == args.len() - 1 {
            return result;
        }
    }

    Expression::Nil
});

/// Executes a builtin command.
pub const BUILTIN: NativeFunction = builtin!(args, frame, streams, {
    // Return Nil if no builtin name is given.
    if args.is_empty() {
        return Expression::Nil;
    }

    if let Some(name) = args[0].value() {
        if let Some(builtin) = lookup(name) {
            return interpreter::native_function_call(builtin, &args[1..], frame, streams);
        }
    }

    Expression::Nil
});

/// Execute an expression, capturing its standard output and returning it as a value.
pub const CAPTURE: NativeFunction = builtin!(lazy args, frame, streams, {
    use std::io::{BufRead, BufReader};
    use std::thread;

    // Set up a new IO context with a piped stdout so we can capture it.
    let (write, read) = io::pipe();
    let mut captured_streams = streams.clone();
    captured_streams.stdout = write;

    // Execute the arguments as an expression in the background.
    let expr = Expression::List(args.to_vec());
    let mut frame = frame.clone();

    thread::spawn(move || {
        interpreter::execute(expr, &mut frame, &mut captured_streams);
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
});

/// Change the current directory.
pub const CD: NativeFunction = builtin!(args, _, _, {
    use std::env;

    if !args.is_empty() {
        if let Some(path) = args[0].value() {
            env::set_current_dir(path).unwrap();
        }
    }

    Expression::Nil
});

/// Executes an external command.
pub const COMMAND: NativeFunction = builtin!(args, _, streams, {
    use exec;

    // Return Nil if no command name is given.
    if args.is_empty() {
        return Expression::Nil;
    }

    // Create a command for the given program name.
    if let Some(command_name) = args[0].value() {
        let mut command = exec::build_external_command(command_name, &args[1..], streams);

        // Run the command in a child process.
        let status = match command.status() {
            Ok(v) => v,
            Err(e) => {
                println!("error running external command '{}': {}", command_name, e);
                return Expression::Nil;
            }
        };

        // Return the exit code.
        let status_string = format!("{}", status.code().unwrap_or(0));
        Expression::atom(status_string)
    } else {
        Expression::Nil
    }
});

pub const CRUSH: NativeFunction = builtin!(args, frame, streams, {
    use std::env;

    if let Ok(path) = env::current_exe() {
        let mut command_args = Vec::with_capacity(args.len() + 1);

        command_args.push(Expression::Atom(path.to_string_lossy().into_owned().into()));
        command_args.extend_from_slice(args);

        interpreter::native_function_call(COMMAND, &command_args, frame, streams)
    } else {
        Expression::Nil
    }
});

/// Define a new function.
pub const DEF: NativeFunction = builtin!(lazy args, frame, streams, {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // First argument is the function name.
    if let Some(name) = interpreter::execute(&args[0], frame, streams).value() {
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
                    if let Some(param_name) = interpreter::execute(param_expr, frame, streams).value() {
                        params.push(param_name.to_owned());
                    }
                }
            }
        }

        // Create the function.
        interpreter::create_function(name, params, body);
    }

    Expression::Nil
});

pub const ENV: NativeFunction = builtin!(args, frame, streams, {
    use std::env;
    use std::io::Write;

    // If no arguments are given, print all variables.
    if args.is_empty() {
        for (name, value) in env::vars() {
            writeln!(streams.stdout, "{} {}", name, value);
        }
    }

    // If one argument is given, lookup and return the value of an environment variable with that name.
    if args.len() == 1 {
        if let Some(name) = interpreter::execute(&args[0], frame, streams).value() {
            if let Ok(value) = env::var(name) {
                return Expression::atom(value);
            }
        }
    }

    Expression::Nil
});

/// Test if all arguments are equal.
pub const EQUAL: NativeFunction = builtin!(args, _, _, {
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

    Expression::TRUE
});

/// Replace the current process with a new command.
pub const EXEC: NativeFunction = builtin!(args, _, streams, {
    use exec;
    use std::os::unix::process::CommandExt;

    if args.len() >= 1 {
        if let Some(command_name) = args[0].value() {
            let mut command = exec::build_external_command(command_name, &args[1..], streams);
            command.exec();
        }
    }

    Expression::Nil
});

/// Exits the current shell.
pub const EXIT: NativeFunction = builtin!(args, _, _, {
    use exit;

    *exit::flag() = true;

    Expression::Nil
});

/// Return the first element of a list.
pub const FIRST: NativeFunction = builtin!(args, _, _, {
    if args.len() > 0 {
        if let Some(items) = args[0].items() {
            if !items.is_empty() {
                return items[0].clone();
            }
        }
    }

    Expression::Nil
});

pub const HELP: NativeFunction = builtin!(args, _, streams, {
    use std::io::Write;

    writeln!(streams.stdout, "<PLACEHOLDER TEXT>").unwrap();

    Expression::Nil
});

/// If the first argument is truthy, the second argument is executed. Otherwise the third argument is executed.
pub const IF: NativeFunction = builtin!(lazy args, frame, streams, {
    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // Determine if the first argument is truthy.
    let condition_expr = interpreter::execute(&args[0], frame, streams);
    let truthy = condition_expr.is_truthy();

    // Evaluate the appropriate expression arm.
    if truthy {
        if args.len() >= 2 {
            interpreter::execute(&args[1], frame, streams)
        } else {
            Expression::Nil
        }
    } else {
        if args.len() >= 3 {
            interpreter::execute(&args[2], frame, streams)
        } else {
            Expression::Nil
        }
    }
});

/// Returns its arguments as a list.
pub const LIST: NativeFunction = builtin!(args, _, _, {
    Expression::List(args.to_vec())
});

pub const NOT: NativeFunction = builtin!(args, _, _, {
    if !args.is_empty() {
        if args[0].is_truthy() {
            Expression::FALSE
        } else {
            Expression::TRUE
        }
    } else {
        Expression::Nil
    }
});

pub const NTH: NativeFunction = builtin!(args, _, _, {
    // We need at least 2 arguments.
    if args.len() < 2 {
        return Expression::Nil;
    }

    if let Some(index) = args[0].parse::<usize>() {
        if let Some(items) = args[1].items() {
            return match items.get(index) {
                Some(item) => item.clone(),
                None => Expression::Nil,
            };
        }
    }

    Expression::Nil
});

/// Form a pipeline between a series of calls and execute them in parallel.
pub const PIPE: NativeFunction = builtin!(lazy args, frame, streams, {
    use std::thread;

    // If no arguments are given, do nothing.
    if args.is_empty() {
        return Expression::Nil;
    }

    // If only on argument is given, just execute it normally.
    if args.len() == 1 {
        return interpreter::execute(&args[0], frame, streams);
    }

    // Multiple arguments are given, so create a series of IO contexts that are chained together.
    let mut contexts = streams.clone().pipeline(args.len() as u16);
    let mut handles = Vec::new();

    for arg in args {
        let expr = arg.clone();
        let mut child_io = contexts.remove(0);
        let mut child_context = frame.clone();

        handles.push(thread::spawn(move || {
            interpreter::execute(expr, &mut child_context, &mut child_io)
        }));
    }

    // Wait for all processes to complete and collect their return values.
    let results = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    Expression::List(results)
});

/// Print the given expressions to standard output. Multiple arguments are separated with a space.
pub const PRINT: NativeFunction = builtin!(args, _, streams, {
    use std::io::Write;

    let mut first = true;

    for arg in args {
        if first {
            write!(streams.stdout, "{}", arg).unwrap();
            first = false;
        } else {
            write!(streams.stdout, " {}", arg).unwrap();
        }
    }

    writeln!(streams.stdout).unwrap();

    Expression::Nil
});

/// Print the current directory.
pub const PWD: NativeFunction = builtin!(_, _, streams, {
    use std::env;
    use std::io::Write;

    writeln!(streams.stdout, "{}", env::current_dir().unwrap().display()).unwrap();

    Expression::Nil
});

/// Returns its arguments as a list unevaluated.
pub const QUOTE: NativeFunction = builtin!(lazy args, _, _, {
    Expression::List(args.to_vec())
});

/// Evaluate the contents of a file and return its result.
pub const SOURCE: NativeFunction = builtin!(args, frame, streams, {
    use std::fs::File;

    // If a filename is given, read from the file. Otherwise read from stdin.
    let expr = if args.is_empty() {
        parser::parse_stream(&mut streams.stdin)
    } else if let Some(filename) = interpreter::execute(&args[0], frame, streams).value() {
        let mut file = File::open(filename).unwrap();
        parser::parse_stream(&mut file)
    } else {
        return Expression::Nil;
    };

    if let Ok(expr) = expr {
        interpreter::execute(expr, frame, streams)
    } else {
        Expression::Nil
    }
});

/// Return the tail of a list.
pub const TAIL: NativeFunction = builtin!(args, _, _, {
    if !args.is_empty() {
        if let Some(items) = args[0].items() {
            return Expression::List(items[1..].to_vec());
        }
    }

    Expression::Nil
});
