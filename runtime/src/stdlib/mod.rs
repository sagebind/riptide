use crate::prelude::*;

mod fs;
mod lang;
mod process;
mod string;

/// This module loader is responsible for loading native and script modules in the standard library.
pub async fn stdlib_loader(_: &mut Fiber, args: &[Value]) -> Result<Value, Exception> {
    let name =
        args.first().and_then(Value::as_string).and_then(|s| s.as_utf8()).ok_or("module name must be a string")?;

    match name {
        "fs" => fs::load(),
        "lang" => lang::load(),
        "process" => process::load(),
        "string" => string::load(),
        _ => Ok(Value::Nil),
    }
}
