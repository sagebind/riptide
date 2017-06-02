use parser::Expression;

pub mod command;
pub mod exec;
pub mod exit;
pub mod help;
pub mod pipe;
pub mod print;


/// A builtin function in native code.
///
/// Builtin functions have the special property of receiving their arguments before they are reduced.
pub type Builtin = fn(&[Expression]);

/// Lookup a builtin function by name.
pub fn get(name: &str) -> Option<Builtin> {
    match name {
        "command" => Some(command::main),
        "exec" => Some(exec::main),
        "exit" => Some(exit::main),
        "help" => Some(help::main),
        "pipe" | "|" => Some(pipe::main),
        "print" | "echo" => Some(print::main),
        _ => None,
    }
}
