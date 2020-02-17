mod eval;
mod fiber;
mod modules;
pub mod builtins;
pub mod closure;
pub mod exceptions;
pub mod foreign;
pub mod scope;
pub mod string;
pub mod table;
pub mod value;

// Re-export syntax crate.
pub mod syntax {
    pub use riptide_syntax::*;
}

pub mod prelude {
    pub use super::exceptions::Exception;
    pub use super::fiber::Fiber;
    pub use super::table::Table;
    pub use super::value::Value;
}

pub use self::{
    fiber::Fiber,
};

/// Initialize a runtime and return a root fiber.
pub async fn init() -> Result<Fiber, exceptions::Exception> {
    use crate::io::IoContext;
    use table::Table;
    use std::{env, time::Instant};

    let start_time = Instant::now();

    let mut fiber = Fiber::new(IoContext::from_process()?);

    // Set up globals
    fiber.globals.set("GLOBALS", fiber.globals.clone());
    fiber.globals.set("env", env::vars().collect::<Table>()); // Isn't that easy?

    // Initialize builtins
    let builtins_table = builtins::get();
    for global in builtins_table.keys() {
        fiber.globals.set(global.clone(), builtins_table.get(global));
    }

    // Register predefined module loaders
    fiber.register_module_loader(crate::stdlib::stdlib_loader);
    fiber.register_module_loader(modules::relative_loader);
    fiber.register_module_loader(modules::system_loader);

    // Execute initialization
    fiber.execute(None, include_str!("init.rip")).await?;

    log::debug!("runtime took {:?} to initialize", start_time.elapsed());

    Ok(fiber)
}
