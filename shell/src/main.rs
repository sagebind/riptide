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

static mut EXIT_FLAG: bool = false;
static mut EXIT_CODE: i32 = 0;

pub fn exit(code: i32) {
    unsafe {
        EXIT_CODE = code;
        EXIT_FLAG = true;
    }
}

fn main() {
    let stdin = fd::stdin();

    if stdin.is_tty() {
        let mut runtime = Runtime::with_stdlib();
        let mut editor = editor::Editor::new();

        loop {
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

            unsafe {
                if EXIT_FLAG {
                    break;
                }
            }
        }
    }

    unsafe {
        process::exit(EXIT_CODE);
    }
}
