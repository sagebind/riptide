//! Implementations of built-in global functions that are always available.

use crate::prelude::*;
use super::modules;
use super::scope::Scope;

pub fn get() -> Table {
    table! {
        "require" => Value::ForeignFn(modules::require.into()),
        "backtrace" => Value::ForeignFn(backtrace.into()),
        "call" => Value::ForeignFn(call.into()),
        "def" => Value::ForeignFn(def.into()),
        "exit" => Value::ForeignFn(exit.into()),
        "include" => Value::ForeignFn(include.into()),
        "list" => Value::ForeignFn(list.into()),
        "nil" => Value::ForeignFn(nil.into()),
        "nth" => Value::ForeignFn(nth.into()),
        "throw" => Value::ForeignFn(throw.into()),
        "try" => Value::ForeignFn(try_fn.into()),
        "typeof" => Value::ForeignFn(type_of.into()),
        "modules" => Value::from(table! {
            "loaders" => Value::List(Vec::new()),
            "loaded" => Value::from(table!()),
        }),
    }
}

/// Binds a value to a new variable.
async fn def(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name required"),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    fiber.set_parent(name, value);

    Ok(Value::Nil)
}

/// Terminate the current process.
async fn exit(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let code = match args.first() {
        Some(&Value::Number(number)) => number as i32,
        _ => 0,
    };

    fiber.exit(code);

    // Throw the exit code as an exception so that the stack will unwind.
    Err(Exception::unrecoverable(code as f64))
}

/// Returns the name of the primitive type of the given arguments.
async fn type_of(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    Ok(args.first().map(Value::type_name).map(Value::from).unwrap_or(Value::Nil))
}

/// Constructs a list from the given arguments.
async fn list(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    Ok(Value::List(args.to_vec()))
}

async fn nth(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let list = match args.get(0).and_then(Value::as_list) {
        Some(s) => s.to_vec(),
        None => throw!("first argument must be a list"),
    };

    let index = match args.get(1).and_then(Value::as_number) {
        Some(s) => s,
        None => throw!("index must be a number"),
    };

    Ok(list.get(index as usize).cloned().unwrap_or(Value::Nil))
}

/// Function that always returns Nil.
async fn nil(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// Throw an exception.
async fn throw(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    match args.first() {
        Some(value) => Err(Exception::from(value.clone())),
        None => Err(Exception::from(Value::Nil)),
    }
}

/// Handle exceptions.
async fn try_fn(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let try_block = match args.first() {
        Some(value) => value,
        None => throw!("block to invoke required"),
    };

    let error_continuation = match args.get(1) {
        Some(value) => value,
        None => throw!("error block required"),
    };

    match fiber.invoke(try_block, &[]).await {
        Ok(value) => Ok(value),
        Err(exception) => {
            // Invoke the catch block.
            let result = fiber.invoke(error_continuation, &[exception.message().clone()]).await;

            // If the exception is unrecoverable, re-throw it anyway.
            if exception.is_unrecoverable() {
                Err(exception)
            } else {
                result
            }
        },
    }
}

async fn call(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    if let Some(function) = args.first() {
        let args = match args.get(1) {
            Some(Value::List(args)) => &args[..],
            _ => &[],
        };

        fiber.invoke(function, args).await
    } else {
        throw!("block to invoke required")
    }
}

async fn include(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    throw!("not implemented");
}

/// Returns a backtrace of the call stack as a list of strings.
async fn backtrace(fiber: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    fn scope_to_value(scope: impl AsRef<Scope>) -> Value {
        let scope = scope.as_ref();
        Value::from(table! {
            "name" => scope.name(),
            "bindings" => scope.bindings.clone(),
        })
    }

    Ok(fiber.backtrace()
        .map(scope_to_value)
        .collect())
}
