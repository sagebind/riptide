extern crate riptide;
extern crate termion;

mod editor;
mod prompt;

use riptide::fd;
use riptide::filemap::FileMap;
use riptide::parse::parse;
use std::process;

pub static mut EXIT_CODE: i32 = 0;

fn main() {
    let stdin = fd::stdin();

    if stdin.is_tty() {
        let mut editor = editor::Editor::new();

        loop {
            let line = editor.read_line();

            match parse(FileMap::buffer(Some("<input>".into()), line)) {
                Ok(ast) => println!("ast: {:?}", ast),
                Err(e) => eprintln!("error: {}", e),
            }
        }
    }

    unsafe {
        process::exit(EXIT_CODE);
    }
}
