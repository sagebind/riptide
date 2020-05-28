//! Implementations of built-in global functions that are always available.

use crate::{
    eval,
    prelude::*,
    scope::Scope,
    string::RipString,
};
use riptide_syntax::source::SourceFile;
use std::convert::TryInto;

pub fn get() -> Table {
    table! {
        "backtrace" => Value::ForeignFn(backtrace.into()),
        "call" => Value::ForeignFn(call.into()),
        "cd" => Value::ForeignFn(cd.into()),
        "exit" => Value::ForeignFn(exit.into()),
        "include" => Value::ForeignFn(include.into()),
        "load" => Value::ForeignFn(load.into()),
        "nil" => Value::ForeignFn(nil.into()),
        "nth" => Value::ForeignFn(nth.into()),
        "pwd" => Value::ForeignFn(pwd.into()),
        "throw" => Value::ForeignFn(throw.into()),
        "try" => Value::ForeignFn(try_fn.into()),
        "typeof" => Value::ForeignFn(type_of.into()),
    }
}

/// Changes the current working directory of the current process.
async fn cd(_fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let dir = match args.first() {
        Some(value) => value.to_string().into(),
        None => directories::BaseDirs::new().unwrap().home_dir().to_owned(),
    };

    // TODO: Should fibers have independent working directories?
    std::env::set_current_dir(dir)?;

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

/// Parse a string as code, returning it as an executable closure.
async fn load(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let script: RipString = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("first argument must be a string"),
    };

    let script: String = script.try_into().map_err(|e: std::string::FromUtf8Error| e.to_string())?;
    let file = SourceFile::named("<dynamic>", script);

    eval::compile(fiber, file).map(Value::from)
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

async fn pwd(fiber: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    Ok(fiber.current_dir())
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
