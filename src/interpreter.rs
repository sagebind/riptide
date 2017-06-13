//! The script language interpreter engine. This is where the magic happens.
//!
//! The interpreter implementation is not incredibly complex, but it does have some neat features:
//! - Stateless interpreter using reentrant recursion.
//! - Tail-call optimization.
//! - Lazy parameter evaluation.
//! - Thread-safe global and lexical bindings.
//!
//! It should be noted that expressions are usually eagerly evaluated. Parameters may be lazily evaluated when calling
//! special builtins called "macros".
use builtins;
use io::Streams;
use parser::{Expression, SourceLocation};
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

/// A function implemented in native code.
///
/// Native functions can receive their arguments either eagerly evaluated (like user functions), or lazily evaluated
/// (like a macro).
pub struct NativeFunction {
    /// Whether this function's arguments are evaluated eagerly or lazily.
    pub lazy_args: bool,

    /// Pointer to the native function body.
    pub ptr: fn(&[Expression], &mut StackFrame, &mut Streams) -> Result<Expression, Exception>,
}

/// A wrapper around a value that can be raised as an error.
#[derive(Debug)]
pub struct Exception {
    pub value: Expression,
}


lazy_static! {
    /// Symbol table for global bindings.
    static ref TABLE: Mutex<HashMap<String, Arc<UserFunction>>> = Mutex::new(HashMap::new());
}

/// Lookup a global binding.
pub fn lookup_function<S>(name: S) -> Option<Arc<UserFunction>>
    where S: AsRef<str>
{
    let table = TABLE.lock().unwrap();
    table.get(name.as_ref()).cloned()
}

/// Create a new global binding.
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


/// A single frame in the interpreter call stack.
///
/// Interestingly, this is also all the state that is needed for the interpreter. Passing frames between method calls is
/// enough to mesh everything together.
///
/// Why is this refcounted? Think about a function call. It is possible for a function body to execute many pipelines
/// and subshells. Since arguments are evaluated lazily, an expression may need to get evaluated in a different thread
/// than the one that initially invoked the function.
#[derive(Clone)]
pub struct StackFrame {
    /// Reference to the function being executed.
    pub function: Option<Arc<UserFunction>>,

    /// Arguments passed in to the current function call. These are the original, unevaluated values.
    pub args: Arc<Vec<Expression>>,

    /// Symbol table for expanded arguments and local values. This is not a hashmap to avoid allocating a heavy hashmap
    /// for each stack frame. The number of locals should typically be small enough that it does not affect performance
    /// too badly.
    // TODO: Maybe use a Judy array?
    pub symbol_table: Arc<Vec<(String, Expression)>>,
}

impl StackFrame {
    pub fn new() -> Self {
        StackFrame {
            function: None,
            args: Arc::new(Vec::new()),
            symbol_table: Arc::new(Vec::new()),
        }
    }
}


/// Execute multiple expressions in sequence and collect the results.
pub fn execute_all(exprs: &[Expression], frame: &mut StackFrame, streams: &mut Streams) -> Result<Vec<Expression>, Exception> {
    let mut results = Vec::with_capacity(exprs.len());

    for expr in exprs {
        results.push(execute(expr, frame, streams)?);
    }

    Ok(results)
}

/// Execute an expression.
///
/// Can take an expression either as a reference or owned.
pub fn execute<'e, E>(expr: E, mut frame: &mut StackFrame, streams: &mut Streams) -> Result<Expression, Exception>
    where E: Into<Cow<'e, Expression>>
{
    let mut next_expr = expr.into();
    let mut tail_frame;

    loop {
        let current_expr = next_expr;

        // Expression is a function call or macro.
        if let Some(args) = current_expr.items() {
            // Prepare to execute a function call. First argument is the function name, which is always eagerly
            // evaluated. Here we execute this using recursion.
            let function_expr = execute(&args[0], frame, streams)?;

            // TODO: Lambdas are not implemented. Assume this is a function name.
            let function_name = function_expr.value().expect("lambdas are not implemented");

            // Check if the function is a local binding.
            for &(ref name, ref value) in frame.symbol_table.iter() {
                if name == function_name {
                    return Ok(value.clone());
                }
            }

            // Check if the function is a global binding.
            if let Some(function) = lookup_function(function_name) {
                // Set the user function to be the next one executed. Doing this lets us avoid another recursive
                // call here (tail-call optimization).
                next_expr = function.body.clone().into();

                // Evaluate the arguments and form a symbol table.
                let mut symbols = Vec::new();
                for (i, arg) in args[1..].iter().enumerate() {
                    let evaluated = execute(arg, frame, streams)?;

                    if let Some(param) = function.params.get(i) {
                        symbols.push((param.clone(), evaluated));
                    }
                }

                // Assign any free params to Nil.
                if symbols.len() < function.params.len() {
                    for i in symbols.len() .. function.params.len() {
                        symbols.push((function.params[i].clone(), Expression::Nil));
                    }
                }

                // Create a new stack frame for the function call.
                tail_frame = StackFrame {
                    function: Some(function),
                    args: Arc::new((&args[1..]).to_vec()),
                    symbol_table: Arc::new(symbols),
                };

                unsafe {
                    frame = &mut *(&mut tail_frame as *mut _);
                }

                continue;
            }

            // Check to see if it is a builtin.
            if let Some(builtin) = builtins::lookup(function_name) {
                return native_function_call(builtin, &args[1..], frame, streams);
            }

            // Execute a command.
            return native_function_call(builtins::COMMAND, args, frame, streams);
        }

        // Expression is already fully reduced.
        return Ok(current_expr.into_owned());
    }
}

/// Execute a function call in the global scope by name.
pub fn function_call(name: &str, args: &[Expression], streams: &mut Streams) -> Result<Expression, Exception> {
    let mut frame = StackFrame::new();
    let mut items = Vec::new();

    items.push(Expression::atom(name.to_string()));
    items.extend_from_slice(args);

    execute(Expression::List(items), &mut frame, streams)
}

/// Call a native builtin function.
pub fn native_function_call(function: NativeFunction, args: &[Expression], frame: &mut StackFrame, streams: &mut Streams) -> Result<Expression, Exception> {
    if function.lazy_args {
        (function.ptr)(args, frame, streams)
    } else {
        let evaluated_args = execute_all(args, frame, streams)?;
        (function.ptr)(&evaluated_args, frame, streams)
    }
}
