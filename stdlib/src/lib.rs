extern crate riptide;

use riptide::prelude::*;

mod strings;

pub fn loader(runtime: &mut Runtime, name: &str) -> Result<Value, Exception> {
    match name {
        "strings" => Ok({
            let mut table = Table::new();
            table.set("len", Value::ForeignFunction(strings::len));
            table.into()
        }),
        _ => Ok(Value::Nil),
    }
}

/// Throws an exception if the given value is not truthy. The exception message is taken from the second argument, or
/// is "assertion failed!" if no message is given.
pub fn assert() {}
