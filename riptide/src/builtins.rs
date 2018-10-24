use prelude::*;

/// Binds a value to a new variable or updates an existing variable.
pub fn def(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name required"),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    runtime.scope().set(name, value);

    Ok(Value::Nil)
}

/// Binds a value to a new variable or updates an existing variable.
pub fn defglobal(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name required"),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    runtime.globals().set(name, value);

    Ok(Value::Nil)
}

/// Returns the name of the primitive type of the given arguments.
pub fn type_of(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(args.first()
        .map(Value::type_name)
        .map(Value::from)
        .unwrap_or(Value::Nil))
}

/// Constructs a list from the given arguments.
pub fn list(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(args.to_vec()))
}

/// Function that always returns Nil.
pub fn nil(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// Throw an exception.
pub fn throw(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    match args.first() {
        Some(value) => Err(Exception::from(value.clone())),
        None => Err(Exception::from(Value::Nil)),
    }
}

pub fn call(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if let Some(function) = args.first() {
        let args = match args.get(1) {
            Some(Value::List(args)) => &args[..],
            _ => &[],
        };

        runtime.invoke(function, args)
    } else {
        throw!("block to invoke required")
    }
}

/// Invoke a block. If the block throws an exception, catch it and return it.
pub fn catch(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if let Some(function) = args.first() {
        match runtime.invoke(function, &[]) {
            Ok(_) => Ok(Value::Nil),
            Err(exception) => Ok(exception.into()),
        }
    } else {
        throw!("block to invoke required")
    }
}

/// Return all arguments passed to the current function as a list.
pub fn args(runtime: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(runtime.scope().args().to_vec()))
}

pub fn include(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    throw!("not implemented");
}
