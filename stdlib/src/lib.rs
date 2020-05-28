use riptide_runtime::{
    prelude::*,
    foreign_fn,
};

mod fs;
mod lang;
mod process;
mod string;

pub async fn init(fiber: &mut Fiber) -> Result<(), Exception> {
    fiber.register_native_module("std/fs", foreign_fn!(|_, _| fs::load()));
    fiber.register_native_module("std/lang", foreign_fn!(|_, _| lang::load()));
    fiber.register_native_module("std/process", foreign_fn!(|_, _| process::load()));
    fiber.register_native_module("std/string", foreign_fn!(|_, _| string::load()));

    // Execute initialization
    fiber.execute(None, include_str!("init.rt")).await?;

    Ok(())
}
