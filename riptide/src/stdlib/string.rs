use prelude::*;

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        "len" => Value::ForeignFunction(|_, _| {
            Ok(Value::Nil)
        }),
        "utf8" => Value::ForeignFunction(|_, args| {
            Ok(args.first()
                .and_then(|s| s.as_string())
                .and_then(|s| s.as_utf8())
                .map(Value::from)
                .unwrap_or(Value::Nil))
        }),
    }.into())
}
