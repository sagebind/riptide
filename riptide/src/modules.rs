//! Module system implementation.
//!
//! The module system is relatively minimal in design. Module names are flat strings without hierarchy, and are loaded
//! by a series of _loaders_. Each loader is a function that converts the module name into the module contents if found,
//! or Nil if not found.

use exceptions::Exception;
use runtime::Runtime;
use std::env;
use std::path::*;
use syntax::source::SourceFile;
use value::Value;

/// Builtin function that loads modules by name.
pub fn require(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    if args.is_empty() {
        return Err("module name to require must be given".into());
    }

    let name = args[0]
        .as_string()
        .ok_or("module name must be a string")?;

    if let Some(value) = runtime
        .get_global("modules")
        .and_then(|modules| modules.get("loaded"))
        .and_then(|loaded| loaded.get(name)) {
            return Ok(value);
        }

    debug!("module '{}' not defined, calling loader chain", name);

    if let Some(loaders) = runtime.get_global("modules").and_then(|t| t.get("loaders")) {
        if let Some(loaders) = loaders.as_list() {
            for loader in loaders {
                match runtime.invoke(loader, &[name.clone().into()]) {
                    Ok(Value::Nil) => continue,
                    Err(exception) => return Err(exception),
                    Ok(value) => {
                        runtime.get_global("modules")
                            .unwrap()
                            .get("loaded")
                            .unwrap()
                            .as_table()
                            .unwrap()
                            .set(name.clone(), value.clone());

                        return Ok(value);
                    },
                }
            }
        }
    }

    Err(Exception::from("module not found"))
}

pub fn relative_loader(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// A module loader function that loads modules from system-wide paths.
pub fn system_loader(runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    let name = args.first().and_then(Value::as_string)
        .ok_or("module name must be a string")?;

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
