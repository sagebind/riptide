extern crate clogger;
#[macro_use]
extern crate log;
extern crate riptide;
#[macro_use]
extern crate structopt;
extern crate termion;

mod buffer;
mod editor;

use riptide::fd;
use riptide::prelude::*;
use riptide::syntax::parse;
use riptide::syntax::source::SourceFile;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;


#[derive(Debug, StructOpt)]
struct Options {
    /// Evaluate the specified commands
    #[structopt(short = "c", long = "command")]
    commands: Vec<String>,

    /// Run as a login shell
    #[structopt(short = "l", long = "login")]
    login: bool,

    /// Set the verbosity level
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbosity: usize,

    /// File to execute
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,
}

fn main_2() -> Result<i32, Exception> {
    // Set up logger.
    clogger::init();

    // Parse command line args.
    let options = Options::from_args();

    // Increase log level by the number of -v flags given.
    clogger::set_verbosity(options.verbosity);

    let stdin = fd::stdin();
    let mut runtime = Runtime::default();

    // If at least one command is given, execute those in order and exit.
    if !options.commands.is_empty() {
        for command in options.commands {
            runtime.execute(None, command)?;
        }
    }

    // If a file is given, execute it and exit.
    else if let Some(file) = options.file.as_ref() {
        runtime.execute(None, SourceFile::open(file)?)?;
    }

    // Interactive mode.
    else {
        if stdin.is_tty() {
            let mut editor = editor::Editor::new();

            while !runtime.exit_requested() {
                let line = editor.read_line();

                if !line.is_empty() {
                    match parse(SourceFile::named("<input>", line)) {
                        Ok(ast) => match runtime.invoke_block(&ast, &[]) {
                            Ok(Value::Nil) => {},
                            Ok(value) => println!("{}", value),
                            Err(e) => eprintln!("error: {}", e),
                        },
                        Err(e) => eprintln!("error: {}", e),
                    }
                }
            }
        }
    }

    Ok(runtime.exit_code())
}

fn main() {
    match main_2() {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        },
    }
}
