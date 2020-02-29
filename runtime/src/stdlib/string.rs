use crate::prelude::*;

pub fn load() -> Result<Value, Exception> {
    Ok(table! {
        // "len" => Value::foreign_fn(|_, _| {
        //     async {
        //         Ok(Value::Nil)
        //     }
        // }),
        // "utf8" => Value::foreign_fn(|_, args: &[Value]| {
        //     async {
        //         Ok(args.first()
        //             .and_then(|s| s.as_string())
        //             .and_then(|s| s.as_utf8())
        //             .map(Value::from)
        //             .unwrap_or(Value::Nil))
        //     }
        // }),
    }
    .into())
}
