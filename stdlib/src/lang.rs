use riptide_runtime::{
    prelude::*,
    table,
};
use tokio::io::AsyncWriteExt;

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "VERSION" => Value::from(env!("CARGO_PKG_VERSION")),
        "assert" => Value::foreign_fn(assert),
        "panic" => Value::foreign_fn(panic),
        "print" => Value::foreign_fn(print),
        "println" => Value::foreign_fn(println),
        "eprint" => Value::foreign_fn(eprint),
        "eprintln" => Value::foreign_fn(eprintln),
        "dump" => Value::foreign_fn(dump),
        // "eq" => Value::foreign_fn(|_, args: &[Value]| async {
        //     Ok(args.iter().all_equal().into())
        // }),
    }
    .into())
}

async fn assert(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    unimplemented!();
}

/// Terminates the current process immediately.
async fn panic(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    panic!();
}

/// Print the given values to standard output.
async fn print(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let stdout = fiber.stdout();
    for arg in args.iter() {
        stdout.write_all(arg.to_string().as_bytes()).await?;
    }
    stdout.flush().await?;

    Ok(Value::Nil)
}

/// Print the given values to standard output, followed by a newline.
async fn println(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let stdout = fiber.stdout();
    for arg in args.iter() {
        stdout.write_all(arg.to_string().as_bytes()).await?;
        stdout.write_all(b"\n").await?;
    }

    Ok(Value::Nil)
}

/// Print the given values to standard error.
async fn eprint(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let stderr = fiber.stderr();
    for arg in args.iter() {
        stderr.write_all(arg.to_string().as_bytes()).await?;
    }
    stderr.flush().await?;

    Ok(Value::Nil)
}

/// Print the given values to standard error, followed by a newline.
async fn eprintln(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let stderr = fiber.stderr();
    for arg in args.iter() {
        stderr.write_all(arg.to_string().as_bytes()).await?;
        stderr.write_all(b"\n").await?;
    }

    Ok(Value::Nil)
}

async fn dump(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
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
