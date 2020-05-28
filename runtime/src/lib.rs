use std::{env, time::Instant};

#[macro_use]
mod macros;

mod builtins;
mod closure;
mod eval;
mod exceptions;
mod fiber;
mod foreign;
pub mod io;
mod modules;
mod scope;
mod string;
mod table;
mod value;

pub use crate::{
    exceptions::Exception,
    fiber::Fiber,
    foreign::ForeignFn,
    table::Table,
    value::Value,
};

// Re-export syntax crate.
pub mod syntax {
    pub use riptide_syntax::*;
}

pub mod prelude {
    pub use crate::{
        exceptions::Exception,
        Fiber,
        table::Table,
        value::Value,
    };
}

/// Evaluate a script inside a one-off runtime and return the result of the
/// script.
pub async fn eval(script: &str) -> Result<Value, Exception> {
    let mut fiber = init().await?;
    fiber.execute(None, script).await
}

/// Initialize a runtime and return a root fiber.
pub async fn init() -> Result<Fiber, Exception> {
    use crate::io::IoContext;

    let start_time = Instant::now();

    let mut fiber = Fiber::new(IoContext::from_process()?);

    // Set up globals
    fiber.globals().set("GLOBALS", fiber.globals().clone());
    fiber.globals().set("env", env::vars().collect::<Table>()); // Isn't that easy?

    // Initialize builtins
    let builtins_table = builtins::get();
    for global in builtins_table.keys() {
        fiber.globals().set(global.clone(), builtins_table.get(global));
    }

    // Run the first bootstrap script
    fiber.execute(None, include_str!("init.rt")).await?;

    // Register predefined module loaders
    fiber.register_module_loader(modules::relative_loader);
    fiber.register_module_loader(modules::system_loader);

    log::debug!("runtime took {:?} to initialize", start_time.elapsed());

    Ok(fiber)
}
