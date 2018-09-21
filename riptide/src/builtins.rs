use exceptions::Exception;
use process;
use runtime::*;
use std::io::{stdout, Write};
use value::*;

/// Binds a value to a new variable or updates an existing variable.
pub fn def(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s,
        None => return Err(Exception::from("variable name required")),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    runtime.set(name, value);

    Ok(Value::Nil)
}

/// Spawns a new child process and executes a given block in it.
///
/// Returns the child process PID.
pub fn spawn(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        // let child_interpreter = Runtime::new();
        // child_interpreter.execute(Exp)
    }).unwrap();

    Ok(Value::Number(pid as f64))
}

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
pub fn command(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Executes a shell command, replacing the current process with the new process.
///
/// Does not return.
pub fn exec(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

pub fn print(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }
    stdout().flush().unwrap();

    Ok(Value::Nil)
}

pub fn println(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

/// Returns the name of the primitive type of the given arguments.
pub fn type_of(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(args.iter().map(|arg| {
        Value::from(arg.type_name())
    }).collect()))
}

/// Constructs a list from the given arguments.
pub fn list(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(args.to_vec()))
}

/// Function that always returns Nil.
pub fn nil(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// Throw an exception.
pub fn throw(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    match args.first() {
        Some(value) => Err(Exception::from(value.clone())),
        None => Err(Exception::from(Value::Nil)),
    }
}

/// Invoke a block. If the block throws an exception, catch it and return it.
pub fn catch(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    match args.first() {
        None => Ok(Value::Nil),
        Some(&Value::Block(ref block)) => match runtime.invoke_block(block, &[]) {
            Ok(_) => Ok(Value::Nil),
            Err(exception) => Ok(exception.into()),
        },
        Some(_) => Err(Exception::from("block to invoke required")),
    }
}

/// Return all arguments passed to the current function as a list.
pub fn args(runtime: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(runtime.current_frame().args.to_vec()))
}

pub fn require(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    match args.first() {
        Some(&Value::String(ref string)) => runtime.load_module(string),
        _ => Err(Exception::from("module name required")),
    }
}

pub fn include(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Terminate the current process.
pub fn exit(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let code = match args.first() {
        Some(&Value::Number(number)) => number as i32,
        _ => 0,
    };

    runtime.request_exit(code);

    Ok(Value::Nil)
}
