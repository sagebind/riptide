extern crate log;
extern crate riptide_syntax;
extern crate simple_logging;

use log::LevelFilter;
use riptide_syntax::source::SourceFile;
use std::error::Error;
use std::io;
use std::io::Read;

fn main() -> Result<(), Box<Error>> {
    simple_logging::log_to_stderr(LevelFilter::Debug);

    let mut source = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut source)?;

    let file = SourceFile::buffer(String::from("<stdin>"), source);

    match riptide_syntax::parse(file) {
        Ok(ast) => println!("{:?}", ast),
        Err(e) => eprintln!("error: {:?}", e),
    }

    Ok(())
}
