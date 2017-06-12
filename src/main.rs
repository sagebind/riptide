#![feature(associated_consts)]

#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate termion;
extern crate utf8parse;

mod builtins;
mod editor;
mod exec;
mod interpreter;
mod io;
mod parser;
mod scanner;

use std::process;


/// Control over the process exit status.
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


/// Run a new shell instance.
fn main() {
    let mut streams = io::Streams::inherited();
    let mut frame = interpreter::StackFrame::new();

    // Source the internal init script.
    let script = parser::parse_string(include_str!("init.crush"))
        .expect("error in internal init script");

    if let Some(items) = script.items() {
        interpreter::execute_all(items, &mut frame, &mut streams);
    }

    // If stdin is interactive, use the editor.
    if streams.stdin.is_tty() {
        loop {
            let line = {
                let mut editor = editor::Editor::new(&mut streams);
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

            let result = interpreter::execute(expression, &mut frame, &mut streams);

            // If the return value isn't Nil, print it out for the user.
            if !result.is_nil() {
                println!("{}", result);
            }

            if *exit::flag() {
                break;
            }
        }
    }

    // Stdin isn't interactive, so treat it is just a script input file.
    else {
        match parser::parse_stream(&mut streams.stdin) {
            Ok(expr) => {
                println!("{}", expr);
            }
            Err(e) => {
                println!("error: {}\n    {}:{}:{}",
                         e.kind.description(),
                         streams.name(),
                         e.pos.line,
                         e.pos.column);
            }
        }
    }

    process::exit(*exit::status());
}
