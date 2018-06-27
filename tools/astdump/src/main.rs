extern crate log;
extern crate riptide_syntax;
extern crate simple_logging;

use log::LevelFilter;
use riptide_syntax::source::SourceFile;
use std::error::Error;
use std::io;

fn main() -> Result<(), Box<Error>> {
    simple_logging::log_to_stderr(LevelFilter::Debug);

    let file = SourceFile::file(String::from("<stdin>"), &mut io::stdin())?;

    match riptide_syntax::parse(file) {
        Ok(ast) => println!("{:?}", ast),
        Err(e) => eprintln!("error: {:?}", e),
    }

    Ok(())
}
