//! The script language interpreter engine. This is where the magic happens.
//!
//! The interpreter implementation is not incredibly complex, but it does have some neat features:
//! - Mostly stackless function calls.
//! - Tail-call optimization.
//! - Lazy parameter evaluation.
//! - Thread-safe global and lexical bindings.
//!
//! It should be noted that not expressions are evaluated lazily. The rules for lazy evaluation are as follows:
//! - The first item in a function call (the function name or lambda) is always eagerly evaluated.
//! - Arguments to user-defined functions are not evaluated until "later".
//! - Builtins determine how their arguments are evaluated individually.
//!
//! In theory, anything that does not end up as an argument to a builtin when an entire program is reduced will _never_
//! get evaluated. This is great for performance, but is different than typical Lisp-like languages, which are
//! traditionally strict-evaluation languages.
use builtins;
use io::IO;
use parser::Expression;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};


/// A function defined by the user.
///
/// User functions consist of an argument list, and a body. The body is an expression, which gets executed on every
/// invocation. The argument list is a list of names that are used inside the body. These names actually become function
/// aliases for the expressions passed in at call time. Thus, if an argument is never used, it is never executed.
pub struct UserFunction {
    pub params: Vec<String>,
    pub body: Expression,
}

/// Global table of user-defined functions.
lazy_static! {
    static ref TABLE: Mutex<HashMap<String, Arc<UserFunction>>> = Mutex::new(HashMap::new());
}

/// Lookup a user-defined function by name.
pub fn lookup_function<S>(name: S) -> Option<Arc<UserFunction>>
    where S: AsRef<str>
{
    let table = TABLE.lock().unwrap();
    table.get(name.as_ref()).cloned()
}

/// Create a new user-defined function.
///
/// If a binding already exists for the given name, the old value is replaced.
pub fn create_function<S>(name: S, params: Vec<String>, body: Expression)
    where S: Into<String>
{
    let function = UserFunction {
        params: params,
        body: body,
    };

    let mut table = TABLE.lock().unwrap();
    table.insert(name.into(), Arc::new(function));
}


/// A function execution context.
///
/// This is essentially a stack frame in the interpreter's virtual call stack.
///
/// Why is this refcounted? Think about a function call. It is possible for a function body to execute many pipelines
/// and subshells. Since arguments are evaluated lazily, an expression may need to get evaluated in a different thread
/// than the one that initially invoked the function.
#[derive(Clone)]
pub struct Context {
    /// Reference to the function being executed.
    function: Option<Arc<UserFunction>>,

    /// Arguments passed in to the current function call.
    args: Arc<Vec<Expression>>,
}

/// Execute an expression.
pub fn execute(expr: &Expression, io: &mut IO) -> Expression {
    let context = Context {
        function: None,
        args: Arc::new(Vec::new()),
    };

    execute_function_call(expr, &context, io)
}

/// Executes an expression as a function call.
///
/// This is a nearly-stackless implementation using tail recursion and a call stack trampoline. The call stack for user
/// functions is always unrolled, but builtins are still implemented using recursion.
pub fn execute_function_call(expr: &Expression, context: &Context, io: &mut IO) -> Expression {
    // Hold a reference to the next expression to execute. Rust can't solve non-lexical borrows, so we're just using a
    // regular pointer.
    let mut expr_ptr = expr as *const Expression;
    let mut expr: Expression;

    let mut context_ptr = context as *const _;
    let mut context: Context;

    loop {
        let expr_ref = unsafe {&*expr_ptr};
        let context_ref = unsafe {&*context_ptr};

        match expr_ref.items() {
            // Expression is already reduced.
            None => return expr_ref.clone(),

            // Expression is a function call.
            Some(args) => {
                // Prepare to execute a function call. First argument is the function name, which is always eagerly
                // evaluated. Here we execute this using recursion.
                let f_expr = execute_function_call(&args[0], context_ref, io);

                // TODO: Lambdas are not implemented. Assume this is a function name.
                let f_name = f_expr.value().expect("lambdas are not implemented");

                // Check if this is the name of a parent function argument that should be expanded.
                if let Some(ref current_function) = context_ref.function {
                    for (i, param) in current_function.params.iter().enumerate() {
                        // Break early if less arguments were passed.
                        if i >= context_ref.args.len() {
                            break;
                        }

                        if param == f_name {
                            return context_ref.args[i].clone();
                        }
                    }
                }

                // Determine the function to be executed by name. First look up a user defined function.
                if let Some(function) = lookup_function(f_name) {
                    // Set the user function to be the next one executed.
                    expr = function.body.clone();
                    expr_ptr = &expr;

                    // Create a new execution context for the function call.
                    context = Context {
                        function: Some(function),
                        args: Arc::new((&args[1..]).to_vec()),
                    };
                    context_ptr = &context;

                    continue;
                }

                // Not a function, check to see if it is a builtin.
                else if let Some(builtin) = builtins::lookup(f_name) {
                    return builtin(&args[1..], context_ref, io);
                }

                // Execute a command.
                else {
                    return builtins::command(args, context_ref, io);
                }
            }
        }
    }
}

/// Execute multiple expressions in sequence, returning all results in a list.
pub fn execute_all(exprs: &[Expression], io: &mut IO) -> Expression {
    let results = exprs.iter().map(|expr| {
        execute(expr, io)
    }).collect();

    Expression::List(results)
}
