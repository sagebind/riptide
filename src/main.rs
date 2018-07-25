extern crate lua;
extern crate nix;
extern crate rustyline;

mod builtins;
mod runtime;

use rustyline::error::ReadlineError;
use std::process::exit;


const DEFAULT_PROMPT: &str = "$ ";


fn main() {
    let mut runtime = runtime::Runtime::new();
    let mut editor = rustyline::Editor::<()>::new();

    loop {
        if builtins::exit::exit_requested() {
            break;
        }

        let prompt = runtime.get_prompt().unwrap_or(DEFAULT_PROMPT.to_string());

        match editor.readline(&prompt) {
            Ok(line) => {
                editor.add_history_entry(line.as_ref());
                runtime.eval(&line);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    exit(*builtins::exit::exit_code());
}
