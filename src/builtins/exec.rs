use builtins;
use parser::Expression;
use std::os::unix::process::CommandExt;


/// Replace the current process with a new command.
pub fn main(args: &[Expression]) {
    if let Some(mut command) = builtins::command::build_command(args) {
        command.exec();
    }
}
