extern crate riptide;
extern crate termion;

mod buffer;
mod editor;

use riptide::fd;
use riptide::filemap::FileMap;
use riptide::parse::parse;
use riptide::runtime::value::Value;
use riptide::runtime::Runtime;
use std::process;

fn main() {
    let stdin = fd::stdin();
    let mut runtime = Runtime::with_stdlib();

    if stdin.is_tty() {
        let mut editor = editor::Editor::new();

        while !runtime.exit_requested() {
            let line = editor.read_line();

            if !line.is_empty() {
                match parse(FileMap::buffer(Some("<input>".into()), line)) {
                    Ok(ast) => {
                        println!("ast: {:?}", ast);

                        match runtime.execute_block(&ast, &[]) {
                            Ok(Value::Nil) => {},
                            Ok(value) => println!("{:?}", value),
                            Err(e) => eprintln!("error: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("error: {}", e),
                }
            }
        }
    }

    process::exit(runtime.exit_code());
}
