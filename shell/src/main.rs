use log::*;
use riptide::fd;
use riptide::prelude::*;
use riptide::syntax::source::SourceFile;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

mod buffer;
mod editor;

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

fn main_impl() -> Result<i32, Exception> {
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
                    match runtime.execute(Some("main"), SourceFile::named("<input>", line)) {
                        Ok(Value::Nil) => {}
                        Ok(value) => println!("{}", value),
                        Err(e) => eprintln!("error: {}", e),
                    }
                }
            }
        }
    }

    Ok(runtime.exit_code())
}

fn main() {
    match main_impl() {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}
