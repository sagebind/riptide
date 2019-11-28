use crate::runtime::prelude::*;
use std::io::{stdout, Write};

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "VERSION" => Value::from(env!("CARGO_PKG_VERSION")),
        "assert" => Value::foreign_fn(assert),
        "panic" => Value::foreign_fn(panic),
        "print" => Value::foreign_fn(print),
        "println" => Value::foreign_fn(println),
        "dump" => Value::foreign_fn(dump),
        "exit" => Value::foreign_fn(exit),
        // "eq" => Value::foreign_fn(|_, args: &[Value]| async {
        //     Ok(args.iter().all_equal().into())
        // }),
    }
    .into())
}

async fn assert(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Terminates the current process immediately.
async fn panic(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    panic!();
}

/// Print the given values to standard output.
async fn print(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }
    stdout().flush().unwrap();

    Ok(Value::Nil)
}

/// Print the given values to standard output, followed by a newline.
async fn println(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

async fn dump(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    fn dump(value: &Value, indent: usize, depth: usize) {
        match value {
            Value::List(items) => {
                println!("{:indent$}[", "", indent = indent);
                for item in items {
                    if depth > 0 {
                        dump(item, indent + 4, depth - 1);
                    } else {
                        println!("{:indent$}...", "", indent = indent + 4);
                    }
                }
                println!("{:indent$}]", "", indent = indent);
            }
            Value::Table(table) => {
                println!("{:indent$}[", "", indent = indent);
                for key in table.keys() {
                    println!("{:indent$}{:?} =>", "", key, indent = indent + 4);
                    if depth > 0 {
                        dump(&table.get(key), indent + 4, depth - 1);
                    } else {
                        println!("{:indent$}...", "", indent = indent + 4);
                    }
                }
                println!("{:indent$}]", "", indent = indent);
            }
            value => println!("{:indent$}{:?}", "", value, indent = indent),
        }
    }

    for arg in args.iter() {
        dump(arg, 0, 3);
    }

    Ok(Value::Nil)
}

/// Terminate the current process.
async fn exit(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let code = match args.first() {
        Some(&Value::Number(number)) => number as i32,
        _ => 0,
    };

    runtime.exit(code);

    Ok(Value::Nil)
}
