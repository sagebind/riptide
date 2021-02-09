use riptide_runtime::prelude::*;

mod fs;
mod lang;
mod process;
mod string;

pub async fn init(fiber: &mut Fiber) -> Result<(), Exception> {
    fiber.register_native_module("std/fs", fs::load);
    fiber.register_native_module("std/lang", lang::load);
    fiber.register_native_module("std/process", process::load);
    fiber.register_native_module("std/string", string::load);

    // Execute initialization
    fiber.execute(None, include_str!("init.rt")).await?;

    Ok(())
}
