//! The Riptide programming language interpreter.

#![allow(dead_code)]

use crate::{
    runtime::prelude::*,
    runtime::syntax::source::SourceFile,
    shell::Editor,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
};
use structopt::StructOpt;

#[macro_use]
mod macros;

mod io;
mod logger;
mod pipes;
mod runtime;
mod shell;
mod stdlib;

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

    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// File to execute
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,

    /// Open a session in private mode.
    ///
    /// In private mode, session history is kept independent from other sessions
    /// and is stored only in memory. All history generated during a private
    /// session will be forgotten when the session terminates.
    #[structopt(long = "private")]
    private: bool,
}

impl Options {
    fn log_level_filter(&self) -> log::LevelFilter {
        if self.quiet {
            log::LevelFilter::Off
        } else {
            match self.verbosity {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            }
        }
    }
}

#[tokio::main(basic_scheduler)]
async fn main() {
    log_panics::init();
    logger::init();

    // Parse command line args.
    let options = Options::from_args();

    // Adjust logging settings based on args.
    log::set_max_level(options.log_level_filter());

    let stdin = std::io::stdin();

    let mut runtime = Runtime::default();

    // If at least one command is given, execute those in order and exit.
    if !options.commands.is_empty() {
        for command in options.commands {
            match runtime.execute(None, command).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{}", e);
                    runtime.exit(1);
                    break;
                }
            }
        }
    }
    // If a file is given, execute it and exit.
    else if let Some(file) = options.file.as_ref() {
        execute_file(&mut runtime, file).await;
    }
    // Interactive mode.
    else if atty::is(atty::Stream::Stdin) {
        interactive_main(&mut runtime).await;
    }
    // Execute stdin
    else {
        log::trace!("stdin is not a tty");
        execute_stdin(&mut runtime, stdin).await;
    }

    // End this process with a particular exit code if specified.
    if let Some(exit_code) = runtime.exit_code() {
        log::trace!("exit({})", exit_code);
        exit(exit_code);
    }
}

async fn execute_file(runtime: &mut Runtime, path: impl AsRef<Path>) {
    let path = path.as_ref();
    let source = match SourceFile::open(path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("opening file {:?}: {}", path, e);
            runtime.exit(exitcode::NOINPUT);
            return;
        }
    };

    if let Err(e) = runtime.execute(None, source).await {
        log::error!("{}", e);
        runtime.exit(1);
    }
}

async fn execute_stdin(runtime: &mut Runtime, mut stdin: impl Read) {
    let mut source = String::new();

    if let Err(e) = stdin.read_to_string(&mut source) {
        log::error!("{}", e);
        runtime.exit(1);
        return;
    }

    if let Err(e) = runtime.execute(None, SourceFile::named("<stdin>", source)).await {
        log::error!("{}", e);
        runtime.exit(1);
    }
}

/// Main loop for an interactive shell session.
///
/// It is also worth noting that this function is infallible. Once set up, the
/// shell ensures that it stays alive until the user actually requests it to
/// exit.
async fn interactive_main(runtime: &mut Runtime) {
    // We want successive commands to act like they are being executed in the
    // same file, so set up a shared scope to execute them in.
    let scope = Rc::new(table!());

    let mut editor = Editor::new(pipes::stdin(), pipes::stdout());

    while runtime.exit_code().is_none() {
        let line = editor.read_line().await;

        if !line.is_empty() {
            match runtime.execute_in_scope(Some("main"), SourceFile::named("<input>", line), scope.clone()).await {
                Ok(Value::Nil) => {}
                Ok(value) => println!("{}", value),
                Err(e) => log::error!("{}", e),
            }
        }
    }
}
