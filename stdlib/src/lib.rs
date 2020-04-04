use riptide_runtime::{
    prelude::*,
    foreign_fn,
};

pub mod fs;
pub mod lang;
pub mod process;
pub mod string;

pub async fn init(fiber: &mut Fiber) -> Result<(), Exception> {
    fiber.register_native_module("fs", foreign_fn!(|_, _| fs::load()));
    fiber.register_native_module("lang", foreign_fn!(|_, _| lang::load()));
    fiber.register_native_module("process", foreign_fn!(|_, _| process::load()));
    fiber.register_native_module("string", foreign_fn!(|_, _| string::load()));

    // Execute initialization
    fiber.execute(None, include_str!("init.rip")).await?;

    Ok(())
}
