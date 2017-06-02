extern crate rustyline;
extern crate utf8parse;

mod builtins;
mod parser;
mod scanner;
mod shell;

use std::process::exit;


fn main() {
    println!("ies init");

    exit(shell::Shell::new().run());
}
