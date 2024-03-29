//! The Riptide programming language interpreter.

#![allow(dead_code)]

use crate::editor::{Editor, ReadLine};
use clap::Parser;
use riptide_runtime::{
    prelude::*,
    syntax::source::SourceFile,
};
use std::{
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
    process::ExitCode,
};
use tokio::signal;

mod buffer;
mod completion;
mod editor;
mod history;
mod logger;
mod os;
mod paths;
mod session;
mod theme;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Options {
    /// Evaluate the specified commands
    #[arg(short = 'c', long = "command")]
    commands: Vec<String>,

    /// Run as a login shell
    #[arg(short = 'l', long = "login")]
    login: bool,

    /// Set the verbosity level
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Silence all output
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// File to execute
    file: Option<PathBuf>,

    /// Open a session in private mode.
    ///
    /// In private mode, session history is kept independent from other sessions
    /// and is stored only in memory. All history generated during a private
    /// session will be forgotten when the session terminates.
    #[arg(long = "private")]
    private: bool,
}

/// Entrypoint of the program. This just does some boring setup and teardown
/// around the real main body of the program.
fn main() -> ExitCode {
    // Parse command line args.
    let options = Options::parse();

    // Initialize logging.
    logger::init(options.verbose.log_level_filter());
    log_panics::init();

    // Create a single-threaded Tokio runtime, which drives the async Riptide
    // runtime without threads.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Run real main and return the exit code.
    rt.block_on(real_main(options)).unwrap_or_default()
}

/// Main program body.
async fn real_main(options: Options) -> Option<ExitCode> {
    let mut fiber = create_runtime().await;

    // If at least one command is given, execute those in order and exit.
    if !options.commands.is_empty() {
        for command in options.commands {
            match fiber.execute(None, command).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{}", e);
                    fiber.exit(1);
                    break;
                }
            }
        }
    }
    // If a file is given, execute it and exit.
    else if let Some(file) = options.file.as_ref() {
        execute_file(&mut fiber, file).await;
    }
    // Interactive mode.
    else if std::io::stdin().is_terminal() {
        interactive_main(&mut fiber, options).await;
    }
    // Execute stdin
    else {
        log::trace!("stdin is not a tty");
        execute_stdin(&mut fiber).await;
    }

    fiber.exit_code().map(|exit_code| {
        if let Ok(exit_code) = u8::try_from(exit_code) {
            exit_code.into()
        } else {
            ExitCode::FAILURE
        }
    })
}

async fn execute_file(fiber: &mut Fiber, path: impl AsRef<Path>) {
    let path = path.as_ref();
    let source = match SourceFile::open(path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("opening file {:?}: {}", path, e);
            fiber.exit(exitcode::NOINPUT);
            return;
        }
    };

    if let Err(e) = fiber.execute(None, source).await {
        log::error!("{}", e);
        fiber.exit(1);
    }
}

async fn execute_stdin(fiber: &mut Fiber) {
    let mut stdin = std::io::stdin();
    let mut source = String::new();

    if let Err(e) = stdin.read_to_string(&mut source) {
        log::error!("{}", e);
        fiber.exit(exitcode::IOERR);
        return;
    }

    if let Err(e) = fiber.execute(None, SourceFile::r#virtual("<stdin>", source)).await {
        log::error!("{}", e);
        fiber.exit(1);
    }
}

/// Main loop for an interactive shell session.
///
/// It is also worth noting that this function is infallible. Once set up, the
/// shell ensures that it stays alive until the user actually requests it to
/// exit.
async fn interactive_main(fiber: &mut Fiber, options: Options) {
    let history = if options.private {
        history::History::in_memory().unwrap()
    } else {
        history::History::open_default().unwrap()
    };

    let session = history.create_session();

    // We want successive commands to act like they are being executed in the
    // same file, so set up a shared scope to execute them in.
    let scope = riptide_runtime::table!();

    // Prepare this scope by running an init script in it.
    let interactive = SourceFile::r#virtual("<input>", include_str!("interactive.rt"));
    fiber.execute_in_scope(Some("main"), interactive, scope.clone())
        .await
        .expect("bug in interactive.rt");

    let completer = completion::history::HistoryCompleter::new(history.clone());

    let mut editor = Editor::new(
        fiber.stdin().try_clone().unwrap(),
        fiber.stdout().try_clone().unwrap(),
        history,
        session,
        completer,
    );

    while fiber.exit_code().is_none() {
        match editor.read_line(fiber).await {
            ReadLine::Input(line) => {
                // If this is a blank line, then don't waste time compiling and
                // executing it.
                if line.is_empty() {
                    continue;
                }

                // Execute the requested input and await for it to complete, or for the
                // user to cancel it with Ctrl-C, whichever happens first.
                tokio::select! {
                    _ = signal::ctrl_c() => {
                        // Insert a blank line.
                        println!();
                    }

                    result = fiber.execute_in_scope(Some("main"), SourceFile::r#virtual("<tty>", line), scope.clone()) => match result {
                        Ok(Value::Nil) => {}
                        Ok(value) => {
                            if let Some(values) = value.as_list() {
                                for value in values {
                                    println!("{}", value);
                                }
                            } else {
                                println!("{}", value);
                            }
                        }
                        Err(e) => if fiber.exit_code().is_none() {
                            log::error!("{}", e)
                        }
                    }
                }
            }

            ReadLine::Eof => {
                log::debug!("exit requested via EOF");
                fiber.exit(0);
            },
        }
    }
}

async fn create_runtime() -> Fiber {
    let mut fiber = riptide_runtime::init().await.expect("error in runtime initialization");
    riptide_stdlib::init(&mut fiber).await.expect("error in runtime initialization");
    fiber
}
