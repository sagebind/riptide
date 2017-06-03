#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate termion;
extern crate utf8parse;

mod builtins;
mod editor;
mod functions;
mod interpreter;
mod io;
mod parser;
mod scanner;

use std::process;


pub mod exit {
    /// Get a reference to the global exit status.
    #[inline]
    pub fn status() -> &'static mut i32 {
        static mut STATUS: i32 = 0;

        unsafe {
            &mut STATUS
        }
    }

    /// Get the process exit flag used to request an exit cleanly.
    #[inline]
    pub fn flag() -> &'static mut bool {
        static mut FLAG: bool = false;

        unsafe {
            &mut FLAG
        }
    }
}

fn main() {
    let mut io = io::IO::inherited();

    // Source the internal init script.
    let script = parser::parse_string(include_str!("init.crush"))
        .expect("error in internal init script");
    if let Some(items) = script.items() {
        interpreter::execute_multiple(items, &mut io);
    }

    // If stdin is interactive, use the editor.
    if io.is_tty() {
        loop {
            let line = {
                let mut editor = editor::Editor::new(&mut io);
                editor.read_line()
            };

            // Parse the command line as a script.
            let expression = match parser::parse_string(&line) {
                Ok(e) => e,
                Err(e) => {
                    println!("error: {}\n    <stdin>:{}:{}",
                            e.kind.description(),
                            e.pos.line,
                            e.pos.column);
                    return;
                }
            };

            interpreter::execute(&expression, &mut io);

            if *exit::flag() {
                break;
            }
        }
    }

    // Stdin isn't interactive, so treat it is just a script input file.
    else {
        match parser::parse_stream(&mut io.stdin) {
            Ok(expr) => {
                println!("{}", expr);
            }
            Err(e) => {
                println!("error: {}\n    {}:{}:{}",
                         e.kind.description(),
                         io.name(),
                         e.pos.line,
                         e.pos.column);
            }
        }
    }

    process::exit(*exit::status());
}
