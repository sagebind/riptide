extern crate nix;
extern crate termion;
extern crate utf8parse;

mod builtins;
mod editor;
mod execute;
mod parser;
mod scanner;
mod shell;

use std::process::exit;


/// Get a reference to the global exit status.
#[inline]
pub fn exit_status() -> &'static mut i32 {
    static mut STATUS: i32 = 0;

    unsafe {
        &mut STATUS
    }
}

fn main() {
    let mut shell = shell::Shell::current();

    // If stdin is interactive, use the editor.
    if shell.is_tty() {
        let mut editor = editor::Editor::new(&mut shell);
        *exit_status() = editor.run();
    }

    // Stdin isn't interactive, so treat it is just a script input file.
    else {
        let result = {
            let mut scanner = scanner::ReaderScanner::new(&mut shell.stdin);
            let parser = parser::Parser::new(&mut scanner);

            parser.parse()
        };

        match result {
            Ok(expr) => {
                println!("{}", expr);
            }
            Err(e) => {
                println!("error: {}\n    {}:{}:{}",
                         e.kind.description(),
                         shell.name(),
                         e.pos.line,
                         e.pos.column);
            }
        }
    }

    exit(*exit_status());
}
