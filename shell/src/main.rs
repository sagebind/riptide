#[macro_use]
extern crate log;
extern crate riptide;
extern crate riptide_stdlib;
extern crate riptide_syntax;
extern crate stderrlog;
extern crate termion;

mod buffer;
mod editor;

use riptide::fd;
use riptide::runtime::*;
use riptide::value::Value;
use riptide_syntax::parse;
use riptide_syntax::source::SourceFile;
use std::process;

fn main() {
    stderrlog::new()
        .verbosity(3)
        .init()
        .unwrap();

    let stdin = fd::stdin();
    let mut runtime = RuntimeBuilder::default()
        .module_loader(riptide_stdlib::loader)
        .build();

    if stdin.is_tty() {
        let mut editor = editor::Editor::new();

        while !runtime.exit_requested() {
            let line = editor.read_line();

            if !line.is_empty() {
                match parse(SourceFile::buffer(Some("<input>".into()), line)) {
                    Ok(ast) => {
                        debug!("ast: {:?}", ast);
                        match runtime.invoke_block(&ast, &[]) {
                            Ok(Value::Nil) => {},
                            Ok(value) => println!("{}", value),
                            Err(e) => eprintln!("error: {}", e),
                        }
                    },
                    Err(e) => eprintln!("error: {}", e),
                }
            }
        }
    }

    process::exit(runtime.exit_code());
}
