use builtins;
use functions::{self, Function};
use io::IO;
use std::borrow::Cow;
use std::cmp;
use std::fmt;


#[derive(Clone, Debug)]
/// Abstract representation of an expression. An expression can either be an atom (string), or a list of expressions
/// surrounded by parenthesis.
pub enum Expression {
    /// A single value.
    Atom(Cow<'static, str>),

    /// An empty list.
    Nil,

    /// A list of expressions.
    List(Vec<Expression>),
}

impl Expression {
    pub const TRUE: Self = Expression::Atom(Cow::Borrowed("true"));
    pub const FALSE: Self = Expression::Atom(Cow::Borrowed("false"));

    /// Create a new atom.
    pub fn atom<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Expression::Atom(value.into())
    }

    /// Determine if this expression is equivalent to Nil, or ().
    pub fn is_nil(&self) -> bool {
        match self {
            &Expression::List(ref items) => items.is_empty(),
            &Expression::Atom(_) => false,
            &Expression::Nil => true,
        }
    }

    /// Determine if this expression is considered a truthy value.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Expression::List(ref items) => !items.is_empty(),
            &Expression::Atom(ref value) => !(value == "0" || value.is_empty() || value.to_lowercase() == "false"),
            &Expression::Nil => false,
        }
    }

    /// If this is an atom expression, get its value.
    pub fn value(&self) -> Option<&str> {
        if let &Expression::Atom(ref s) = self {
            Some(s)
        } else {
            None
        }
    }

    // If this is a non-empty list, return a reference to its contents.
    pub fn items(&self) -> Option<&[Expression]> {
        match self {
            &Expression::List(ref items) if items.len() > 0 => Some(items),
            _ => None,
        }
    }
}

impl cmp::PartialEq for Expression {
    fn eq(&self, rhs: &Self) -> bool {
        (self.is_nil() && rhs.is_nil()) || (self.value() == rhs.value())
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Expression::Atom(ref s) => write!(f, "{}", s),
            &Expression::Nil => write!(f, "()"),
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
        }
    }
}


/// Executes an expression as a function call.
///
/// This is a nearly-stackless implementation using tail recursion and a trampoline.
pub fn execute(expr: &Expression, io: &mut IO) -> Expression {
    let mut expr = Cow::Borrowed(expr);
    let mut next_expr: Expression;

    loop {
        match expr.items() {
            // Expression is already reduced.
            None => return expr.clone().into_owned(),

            // Expression is a function call.
            Some(args) => {
                // Prepare to execute a function call. First argument is the function name, which is always eagerly
                // evaluated. Here we execute this using recursion.
                let f_expr = execute(&args[0], io);

                // TODO: Lambdas are not supported. Assume this is a function name.
                let f_name = f_expr.value().expect("lambdas are not supported");

                // Determine the function to be executed by name. First look up a user defined function.
                if let Some(f) = functions::lookup(f_name) {
                    next_expr = f.body.clone();
                }

                // Try to execute a builtin.
                else if let Some(f) = builtins::lookup(f_name) {
                    return f.execute(&args[1..], io);
                }

                // Execute a command.
                else {
                    return builtins::command(args, io);
                }
            }
        }

        expr = Cow::Owned(next_expr);
    }
}

/// Execute multiple expressions in sequence, returning all results in a list.
pub fn execute_all(exprs: &[Expression], io: &mut IO) -> Expression {
    let results = exprs.iter().map(|expr| {
        execute(expr, io)
    }).collect();

    Expression::List(results)
}
