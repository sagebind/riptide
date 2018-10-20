use prelude::*;
use std::io::{stdout, Write};

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "VERSION" => Value::from(env!("CARGO_PKG_VERSION")),
        "assert" => Value::ForeignFunction(assert),
        "panic" => Value::ForeignFunction(panic),
        "print" => Value::ForeignFunction(print),
        "println" => Value::ForeignFunction(println),
        "dump" => Value::ForeignFunction(dump),
        "exit" => Value::ForeignFunction(exit),
    }.into())
}

fn assert(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Terminates the current process immediately.
fn panic(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    panic!();
}

/// Print the given values to standard output.
fn print(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }
    stdout().flush().unwrap();

    Ok(Value::Nil)
}

/// Print the given values to standard output, followed by a newline.
fn println(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

fn dump(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    fn dump(value: &Value, indent: usize, depth: usize) {
        match value {
            Value::List(items) => {
                println!("{:indent$}[", "", indent=indent);
                for item in items {
                    if depth > 0 {
                        dump(item, indent + 4, depth - 1);
                    } else {
                        println!("{:indent$}...", "", indent=indent + 4);
                    }
                }
                println!("{:indent$}]", "", indent=indent);
            },
            Value::Table(table) => {
                println!("{:indent$}{{", "", indent=indent);
                for key in table.keys() {
                    println!("{:indent$}{:?} =>", "", key, indent=indent + 4);
                    if depth > 0 {
                        dump(&table.get(key), indent + 4, depth - 1);
                    } else {
                        println!("{:indent$}...", "", indent=indent + 4);
                    }
                }
                println!("{:indent$}}}", "", indent=indent);
            },
            value => println!("{:indent$}{}", "", value, indent=indent),
        }
    }

    for arg in args.iter() {
        dump(arg, 0, 3);
    }

    Ok(Value::Nil)
}

/// Terminate the current process.
fn exit(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let code = match args.first() {
        Some(&Value::Number(number)) => number as i32,
        _ => 0,
    };

    runtime.exit(code);

    Ok(Value::Nil)
}
