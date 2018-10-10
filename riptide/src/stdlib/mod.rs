use exceptions::Exception;
use runtime::Runtime;
use value::Value;

mod lang;
mod process;

/// This module loader is responsible for loading native and script modules in the standard library.
pub fn stdlib_loader(_: &mut Runtime, name: &str) -> Result<Value, Exception> {
    match name {
        "lang" => Ok(table! {
            "VERSION" => Value::from(env!("CARGO_PKG_VERSION")),
            "assert" => Value::ForeignFunction(lang::assert),
            "panic" => Value::ForeignFunction(lang::panic),
            "print" => Value::ForeignFunction(lang::print),
            "println" => Value::ForeignFunction(lang::println),
            "exit" => Value::ForeignFunction(lang::exit),
        }.into()),
        "process" => Ok(table! {
            "command" => Value::ForeignFunction(process::command),
            "exec" => Value::ForeignFunction(process::exec),
            "spawn" => Value::ForeignFunction(process::spawn),
        }.into()),
        "string" => Ok(table! {
            "len" => Value::ForeignFunction(|_, _| {
                Ok(Value::Nil)
            }),
        }.into()),
        _ => Ok(Value::Nil),
    }
}
