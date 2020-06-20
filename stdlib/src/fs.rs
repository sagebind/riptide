use riptide_runtime::{prelude::*, table, throw};
use tokio::{fs::File, io};

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "read" => Value::foreign_fn(read),
        "write" => Value::foreign_fn(write),
        "glob" => Value::foreign_fn(glob),
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

async fn glob(_: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let pattern = match args.first().and_then(Value::as_string) {
        Some(p) => match p.as_utf8() {
            Some(s) => s,
            None => throw!("glob pattern must be a UTF-8 string"),
        },
        None => throw!("glob pattern required"),
    };

    glob::glob(pattern)
        .map(|results| {
            results
                .into_iter()
                .filter_map(|result| match result {
                    Ok(path) => Some(Value::from(path)),
                    Err(e) => {
                        log::warn!("error in glob result: {}", e);
                        None
                    }
                })
                .collect::<Value>()
        })
        .map_err(|e| Exception::from(e.to_string()))
}
