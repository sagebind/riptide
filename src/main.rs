#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate termion;
extern crate utf8parse;

mod builtins;
mod editor;
mod execute;
mod functions;
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

    // If stdin is interactive, use the editor.
    if io.is_tty() {
        loop {
            let line = {
                let mut editor = editor::Editor::new(&mut io);
                editor.read_line()
            };

            // Build a parser.
            let mut scanner = scanner::StringScanner::new(&line);
            let parser = parser::Parser::new(&mut scanner);

            // Parse the command line as a script.
            let expression = match parser.parse() {
                Ok(e) => e,
                Err(e) => {
                    println!("error: {}\n    <stdin>:{}:{}",
                            e.kind.description(),
                            e.pos.line,
                            e.pos.column);
                    return;
                }
            };

            execute::execute(&expression, &mut io);

            if *exit::flag() {
                break;
            }
        }
    }

    // Stdin isn't interactive, so treat it is just a script input file.
    else {
        let result = {
            let mut scanner = scanner::ReaderScanner::new(&mut io.stdin);
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
                         io.name(),
                         e.pos.line,
                         e.pos.column);
            }
        }
    }

    process::exit(*exit::status());
}
