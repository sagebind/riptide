//! Module system implementation.

use crate::{
    prelude::*,
    syntax::source::SourceFile,
    throw,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    env,
    path::*,
};

/// Base trait that allows you to implement a module in entirely native code.
pub trait NativeModule {
    fn init(&self) -> Result<Value, Exception>;
}

impl<F> NativeModule for F
where
    F: Fn() -> Result<Value, Exception>,
{
    fn init(&self) -> Result<Value, Exception> {
        (self)()
    }
}

pub(crate) struct ModuleIndex {
    loaded: RefCell<HashMap<String, Value>>,
    native_modules: RefCell<HashMap<String, Box<dyn NativeModule>>>,
}

impl Default for ModuleIndex {
    fn default() -> Self {
        let index = Self {
            loaded: Default::default(),
            native_modules: Default::default(),
        };

        index.register_native_module("builtins", crate::builtins::load_module);

        index
    }
}

impl ModuleIndex {
    pub(crate) async fn load(&self, fiber: &mut Fiber, name: &str) -> Result<Value, Exception> {
        if let Some(value) = self.loaded.borrow().get(name) {
            return Ok(value.clone());
        }

        if let Some(native_module) = self.native_modules.borrow().get(name) {
            log::debug!("loading native module '{}'", name);

            let value = native_module.init()?;

            self.loaded.borrow_mut().insert(name.to_owned(), value.clone());

            return Ok(value);
        }

        if let Ok(path) = env::var("RIPTIDE_PATH") {
            for path in path.split(':') {
                let mut path = PathBuf::from(path);
                path.push(format!("{}.rt", name));

                if path.exists() {
                    log::debug!("loading module '{}' from '{}'", name, path.display());
                    return fiber.execute(Some(name), SourceFile::open(path)?).await;
                }
            }
        }

        throw!("module '{}' not found", name)
    }

    /// Register a module implemented in native code.
    pub(crate) fn register_native_module<N, M>(&self, name: N, module: M)
    where
        N: Into<String>,
        M: NativeModule + 'static,
    {
        self.native_modules.borrow_mut().insert(name.into(), Box::new(module));
    }
}
