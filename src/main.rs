extern crate rustyline;
extern crate utf8parse;

mod builtins;
mod execute;
mod parser;
mod scanner;
mod shell;

use std::process::exit;


fn main() {
    println!("crush init");

    exit(shell::Shell::new().run());
}
