//! Module system implementation.
//!
//! The module system is relatively minimal in design. Module names are flat strings without hierarchy, and are loaded
//! by a series of _loaders_. Each loader is a function that converts the module name into the module contents if found,
//! or Nil if not found.

use crate::{
    prelude::*,
    foreign::ForeignFn,
    syntax::source::SourceFile,
};
use std::{env, path::*};

pub mod native;

impl Fiber {
    /// Register a module loader.
    pub(crate) fn register_module_loader(&self, loader: impl Into<ForeignFn>) {
        let modules = self.globals().get("modules").as_table().unwrap();
        modules.set(
            "loaders",
            modules.get("loaders").append(loader.into()).unwrap(),
        );
    }

    /// Register a module implemented in native code.
    // TODO: Change type of name.
    pub fn register_native_module(&self, name: &'static str, init: impl Into<ForeignFn>) {
        let init = init.into();

        self.register_module_loader(foreign_fn!(clone init |fiber, args| {
            if let Some(s) = args.first().and_then(Value::as_string) {
                if s == name {
                    return init.call(fiber, vec![]).await;
                }
            }

            Ok(Value::Nil)
        }));
    }
}

pub async fn relative_loader(_: &mut Fiber, _: Vec<Value>) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

/// A module loader function that loads modules from system-wide paths.
pub async fn system_loader(fiber: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
    let name = args.first().and_then(Value::as_string).ok_or("module name must be a string")?;
    log::debug!("loading module '{}' using system loader", name);

    if let Ok(path) = env::var("RIPTIDE_PATH") {
        for path in path.split(':') {
            let mut path = PathBuf::from(path);
            path.push(format!("{}.rip", name));

            if path.exists() {
                return fiber.execute(Some(name.as_utf8().unwrap()), SourceFile::open(path)?).await;
            }
        }
    }

    Ok(Value::Nil)
}
