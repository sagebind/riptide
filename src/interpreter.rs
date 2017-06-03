use builtins;
use functions::{self, Function};
use io::IO;
use std::borrow::Cow;
use std::cmp;
use std::fmt;
use std::os::unix::io::*;
use std::process::{Command, Stdio};


#[derive(Clone, Debug)]
/// Abstract representation of an expression. An expression can either be an atom (string), or a list of expressions
/// surrounded by parenthesis.
pub enum Expression {
    /// A list of expressions.
    List(Vec<Expression>),
    /// A value.
    Atom(Cow<'static, str>),
    /// An empty list. This is equivalent to List with no expressions.
    Nil,
}

impl Expression {
    pub fn True() -> Self {
        Expression::Atom("true".into())
    }

    pub fn False() -> Self {
        Expression::Atom("false".into())
    }

    /// Determine if this expression is considered a truthy value.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Expression::List(ref items) => !items.is_empty(),
            &Expression::Atom(ref value) => !(value == "0" || value.is_empty() || value.to_lowercase() == "false"),
            &Expression::Nil => false,
        }
    }

    // If this is a list, return a slice of its contents.
    pub fn items(&self) -> Option<&[Expression]> {
        if let &Expression::List(ref items) = self {
            Some(items)
        } else {
            None
        }
    }

    /// If this is an atom expression, get its value.
    pub fn atom(&self) -> Option<&str> {
        if let &Expression::Atom(ref s) = self {
            Some(s)
        } else {
            None
        }
    }
}

impl cmp::PartialEq for Expression {
    fn eq(&self, rhs: &Self) -> bool {
        self.atom() == rhs.atom()
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Expression::List(ref v) => {
                write!(f, "(")?;
                let mut first = true;
                for expr in v {
                    if first {
                        write!(f, "{}", expr)?;
                        first = false;
                    } else {
                        write!(f, " {}", expr)?;
                    }
                }
                write!(f, ")")
            },
            &Expression::Atom(ref s) => write!(f, "{}", s),
            &Expression::Nil => write!(f, "()"),
        }
    }
}

/// Execute an expression as a function call.
pub fn execute(expr: &Expression, io: &mut IO) {
    reduce(expr, io);
}

/// Execute multiple expressions in sequence, returning all results in a list.
pub fn execute_multiple(exprs: &[Expression], io: &mut IO) -> Expression {
    let results = exprs.iter().map(|expr| {
        reduce(expr, io)
    }).collect();

    Expression::List(results)
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
                // First look up a user defined function.
                if let Some(f) = functions::lookup(name) {
                    return f.execute(&atoms[1..], io);
                }

                // Try to execute a builtin.
                else if let Some(f) = builtins::lookup(name) {
                    return f.execute(&atoms[1..], io);
                }

                // Execute a command.
                else {
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
    if let Some(expr) = args.first().map(|e| reduce(e, io)) {
        if let Some(name) = expr.atom() {
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

            return Some(command);
        }
    }

    None
}
