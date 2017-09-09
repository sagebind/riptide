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
use expr::*;
use globals;
use io::Streams;
use std::borrow::Cow;
use std::sync::Arc;


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
    pub function: Option<Arc<Function>>,

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
        if let Some(items) = current_expr.as_items() {
            // Prepare to execute a function call. First argument is the function to execute, the remaining items are
            // arguments.
            let mut function_expr = execute(&items[0], frame, streams)?;
            let args = if items.len() > 1 {
                &items[1..]
            } else {
                &[]
            };

            // If the first item is a name, resolve binding names first before we try to execute the item as a function.
            if function_expr.as_value().is_some() {
                let resolved_function_expr;

                'jmp: loop {
                    let function_name = function_expr.as_value().unwrap();

                    // Check for a local binding.
                    for &(ref name, ref value) in frame.symbol_table.iter() {
                        if name == function_name {
                            resolved_function_expr = value.clone();
                            break 'jmp;
                        }
                    }

                    // Check for a global binding.
                    if let Some(value) = globals::get(function_name) {
                        resolved_function_expr = (*value).clone();
                        break;
                    }

                    // Check to see if it is a builtin.
                    if let Some(builtin) = builtins::lookup(function_name) {
                        return do_native_function_call(builtin, args, frame, streams);
                    }

                    // The name means nothing to us, so assume it is the name of a command.
                    return do_native_function_call(builtins::COMMAND, items, frame, streams);
                }

                function_expr = resolved_function_expr;
            }

            // Check if the function expression is a lambda.
            if let Some(function) = function_expr.as_lambda() {
                // Set the user function to be the next one executed. Doing this lets us avoid another recursive
                // call here (tail-call optimization).
                next_expr = function.body.clone().into();

                // Create a new stack frame for the function call.
                tail_frame = prepare_function_call(function, args, frame, streams)?;
                unsafe {
                    frame = &mut *(&mut tail_frame as *mut _);
                }

                continue;
            }

            // If any arguments were given, then someone's trying to call a non-function.
            if !args.is_empty() {
                return Err(Exception {
                    value: Expression::atom(format!("cannot execute {} as a function", function_expr)),
                });
            }

            // The first item is just a value, so return it.
            return Ok(function_expr);
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
    if args.len() > 0 {
        items.extend_from_slice(args);
    }

    execute(Expression::List(items), &mut frame, streams)
}

/// Call a native builtin function.
pub fn do_native_function_call(function: NativeFunction, args: &[Expression], frame: &mut StackFrame, streams: &mut Streams) -> Result<Expression, Exception> {
    if function.lazy_args {
        (function.ptr)(args, frame, streams)
    } else {
        let evaluated_args = execute_all(args, frame, streams)?;
        (function.ptr)(&evaluated_args, frame, streams)
    }
}

/// Prepare a call stack to execute a user function.
fn prepare_function_call(function: Arc<Function>, args: &[Expression], frame: &mut StackFrame, streams: &mut Streams) -> Result<StackFrame, Exception> {
    // Evaluate the arguments and form a symbol table.
    let mut symbols = Vec::new();
    for (i, arg) in args.iter().enumerate() {
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
    Ok(StackFrame {
        function: Some(function.clone()),
        args: Arc::new(args.to_vec()),
        symbol_table: Arc::new(symbols),
    })
}
