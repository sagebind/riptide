use prelude::*;
use std::io::{stdout, Write};

pub fn assert(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Terminates the current process immediately.
pub fn panic(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    panic!();
}

/// Print the given values to standard output.
pub fn print(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }
    stdout().flush().unwrap();

    Ok(Value::Nil)
}

/// Print the given values to standard output, followed by a newline.
pub fn println(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

/// Terminate the current process.
pub fn exit(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let code = match args.first() {
        Some(&Value::Number(number)) => number as i32,
        _ => 0,
    };

    runtime.exit(code);

    Ok(Value::Nil)
}
