//! Module system implementation.
//!
//! The module system is relatively minimal in design. Module names are flat strings without hierarchy, and are loaded
//! by a series of _loaders_. Each loader is a function that converts the module name into the module contents if found,
//! or Nil if not found.

use crate::prelude::*;
use crate::syntax::source::SourceFile;
use log::*;
use std::env;
use std::path::*;

/// Builtin function that loads modules by name.
pub fn require(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if args.is_empty() {
        throw!("module name to require must be given");
    }

    let name = args[0].as_string().ok_or("module name must be a string")?;

    match runtime.globals().get("modules").get("loaded").get(name) {
        Value::Nil => {}
        value => return Ok(value),
    }

    debug!("module '{}' not defined, calling loader chain", name);

    if let Some(loaders) = runtime.globals().get("modules").get("loaders").as_list() {
        for loader in loaders {
            match runtime.invoke(loader, &[name.clone().into()]) {
                Ok(Value::Nil) => continue,
                Err(exception) => return Err(exception),
                Ok(value) => {
                    runtime.globals().get("modules").get("loaded").as_table().unwrap().set(name.clone(), value.clone());

                    return Ok(value);
                }
            }
        }
    }

    throw!("module '{}' not found", name)
}

pub fn relative_loader(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// A module loader function that loads modules from system-wide paths.
pub fn system_loader(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = args.first().and_then(Value::as_string).ok_or("module name must be a string")?;

    if let Ok(path) = env::var("RIPTIDE_PATH") {
        for path in path.split(':') {
            let mut path = PathBuf::from(path);
            path.push(format!("{}.rip", name));

            if path.exists() {
                return runtime.execute(Some(name.as_utf8().unwrap()), SourceFile::open(path)?);
            }
        }
    }

    Ok(Value::Nil)
}
