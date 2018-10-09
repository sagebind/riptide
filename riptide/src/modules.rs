use exceptions::Exception;
use runtime::Runtime;
use std::env;
use std::path::*;
use syntax::source::SourceFile;
use value::Value;

/// Loads modules by name.
pub trait ModuleLoader {
    /// Attempt to load a module by name. Returns `Nil` if the module could not be found.
    fn load(&self, runtime: &mut Runtime, name: &str) -> Result<Value, Exception>;
}

impl<F> ModuleLoader for F where F: Fn(&mut Runtime, &str) -> Result<Value, Exception> {
    fn load(&self, runtime: &mut Runtime, name: &str) -> Result<Value, Exception> {
        (self)(runtime, name)
    }
}

pub fn relative_loader(_: &mut Runtime, _: &str) -> Result<Value, Exception> {
    Ok(Value::Nil)
}

pub fn system_loader(runtime: &mut Runtime, name: &str) -> Result<Value, Exception> {
    if let Ok(path) = env::var("RIPTIDE_PATH") {
        for path in path.split(':') {
            let mut path = PathBuf::from(path);
            path.push(format!("{}.rip", name));

            if path.exists() {
                return runtime.execute(Some(name), SourceFile::open(path)?);
            }
        }
    }

    Ok(Value::Nil)
}
