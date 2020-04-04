use riptide_runtime::{
    prelude::*,
    table,
    throw,
};
use tokio::{fs::File, io};

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "read" => Value::foreign_fn(read),
        "write" => Value::foreign_fn(write),
    }
    .into())
}

async fn read(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let path = match args.first().and_then(Value::as_string) {
        Some(p) => p.as_os_str(),
        None => throw!("file path required"),
    };

    let mut file = File::open(path).await?;
    let count = io::copy(&mut file, fiber.stdout()).await?;

    Ok(Value::from(count))
}

async fn write(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let path = match args.first().and_then(Value::as_string) {
        Some(p) => p.as_os_str(),
        None => throw!("file path required"),
    };

    let mut file = File::create(path).await?;
    let count = io::copy(fiber.stdin(), &mut file).await?;

    Ok(Value::from(count))
}
