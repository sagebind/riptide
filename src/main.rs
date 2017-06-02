extern crate nix;
extern crate termion;
extern crate utf8parse;

mod builtins;
mod execute;
mod parser;
mod scanner;
mod shell;

use std::process::exit;


fn main() {
    let mut shell = shell::Shell::default();
    let status = shell.run();

    exit(status);
}
