use log::*;
use riptide_runtime::fd;
use riptide_runtime::prelude::*;
use riptide_runtime::syntax::source::SourceFile;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;
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

fn run() -> Result<i32, Exception> {
    // Set up logger.
    clogger::init();

    // Parse command line args.
    let options = Options::from_args();

    // Increase log level by the number of -v flags given.
    clogger::set_verbosity(options.verbosity);

    let mut stdin = fd::stdin();
    let mut runtime = Runtime::default();

    // An executor for running the asynchronous runtime.
    let mut executor = futures::executor::LocalPool::default();

    // We want successive commands to act like they are being executed in the
    // same file, so set up a shared scope to execute them in.
    let scope = Rc::new(riptide_runtime::table!());

    // If at least one command is given, execute those in order and exit.
    if !options.commands.is_empty() {
        for command in options.commands {
            executor.run_until(runtime.execute(None, command))?;
        }
    }
    // If a file is given, execute it and exit.
    else if let Some(file) = options.file.as_ref() {
        executor.run_until(runtime.execute(None, SourceFile::open(file)?))?;
    }
    // Interactive mode.
    else if stdin.is_tty() {
        let mut editor = editor::Editor::new();

        while !runtime.exit_requested() {
            let line = editor.read_line();

            if !line.is_empty() {
                match executor.run_until(runtime.execute_in_scope(Some("main"), SourceFile::named("<input>", line), scope.clone())) {
                    Ok(Value::Nil) => {}
                    Ok(value) => println!("{}", value),
                    Err(e) => eprintln!("error: {}", e),
                }
            }
        }
    }
    // Execute stdin
    else {
        let mut source = String::new();
        stdin.read_to_string(&mut source)?;
        executor.run_until(runtime.execute(None, SourceFile::named("<stdin>", source)))?;
    }

    Ok(runtime.exit_code())
}

fn main() {
    match run() {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}
