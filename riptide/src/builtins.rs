//! Implementations of built-in global functions that are always available.

use crate::modules;
use crate::prelude::*;
use crate::runtime::Scope;

pub fn init(runtime: &mut Runtime) {
    runtime.globals().set("require", Value::ForeignFunction(modules::require));
    runtime.globals().set("backtrace", Value::ForeignFunction(backtrace));
    runtime.globals().set("call", Value::ForeignFunction(call));
    runtime.globals().set("catch", Value::ForeignFunction(catch));
    runtime.globals().set("def", Value::ForeignFunction(def));
    runtime.globals().set("export", Value::ForeignFunction(export));
    runtime.globals().set("include", Value::ForeignFunction(include));
    runtime.globals().set("list", Value::ForeignFunction(list));
    runtime.globals().set("nil", Value::ForeignFunction(nil));
    runtime.globals().set("nth", Value::ForeignFunction(nth));
    runtime.globals().set("set", Value::ForeignFunction(set));
    runtime.globals().set("table", Value::ForeignFunction(table));
    runtime.globals().set("table-set", Value::ForeignFunction(table_set));
    runtime.globals().set("throw", Value::ForeignFunction(throw));
    runtime.globals().set("typeof", Value::ForeignFunction(type_of));

    runtime.globals().set("modules", Value::from(table! {
        "loaders" => Value::List(Vec::new()),
        "loaded" => Value::from(table!()),
    }));
}

/// Binds a value to a new variable.
fn def(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name required"),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    runtime.set_parent(name, value);

    Ok(Value::Nil)
}

fn set(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name required"),
    };

    let value = args.get(1).cloned().unwrap_or(Value::Nil);

    runtime.set_parent(name, value);

    Ok(Value::Nil)
}

fn export(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.get(0).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("variable name to export required"),
    };

    let value = args.get(1).cloned()
        .unwrap_or(runtime.get(&name));

    runtime.module_scope().set(name, value);

    Ok(Value::Nil)
}

/// Returns the name of the primitive type of the given arguments.
fn type_of(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(args.first().map(Value::type_name).map(Value::from).unwrap_or(Value::Nil))
}

/// Constructs a list from the given arguments.
fn list(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    Ok(Value::List(args.to_vec()))
}

fn nth(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let list = match args.get(0).and_then(Value::as_list) {
        Some(s) => s.to_vec(),
        None => throw!("first argument must be a list"),
    };

    let index = match args.get(1).and_then(Value::as_number) {
        Some(s) => s,
        None => throw!("index must be a number"),
    };

    Ok(list.get(index as usize).cloned().unwrap_or(Value::Nil))
}

/// Constructs a table from the given arguments.
fn table(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if args.len() & 1 == 1 {
        throw!("an even number of arguments is required");
    }

    let table = table!();
    let mut iter = args.iter();

    while let Some(key) = iter.next() {
        let value = iter.next().unwrap();

        if let Some(key) = key.as_string() {
            table.set(key.clone(), value.clone());
        } else {
            throw!("table key must be a string");
        }
    }

    Ok(table.into())
}

fn table_set(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let table = match args.get(0).and_then(Value::as_table) {
        Some(s) => s.clone(),
        None => throw!("first argument must be a table"),
    };

    let key = match args.get(1).and_then(Value::as_string) {
        Some(s) => s.clone(),
        None => throw!("key must be a string"),
    };

    let value = args.get(2).cloned().unwrap_or(Value::Nil);

    table.set(key, value);

    Ok(Value::Nil)
}

/// Function that always returns Nil.
fn nil(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// Throw an exception.
fn throw(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    match args.first() {
        Some(value) => Err(Exception::from(value.clone())),
        None => Err(Exception::from(Value::Nil)),
    }
}

fn call(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
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
fn catch(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if let Some(function) = args.first() {
        match runtime.invoke(function, &[]) {
            Ok(_) => Ok(Value::Nil),
            Err(exception) => Ok(exception.into()),
        }
    } else {
        throw!("block to invoke required")
    }
}

fn include(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    throw!("not implemented");
}

/// Returns a backtrace of the call stack as a list of strings.
fn backtrace(runtime: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    fn scope_to_value(scope: impl AsRef<Scope>) -> Value {
        let scope = scope.as_ref();
        Value::from(table! {
            "name" => scope.name(),
            "args" => scope.args(),
            "bindings" => scope.bindings.clone(),
            "parent" => scope.parent.as_ref().map(scope_to_value).unwrap_or(Value::Nil),
        })
    }

    Ok(runtime.stack
        .iter()
        .rev()
        .map(scope_to_value)
        .collect())
}
