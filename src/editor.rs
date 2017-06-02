use builtins;
use execute;
use parser;
use scanner;
use shell::Shell;
use std::io::{Read, Write};
use termion::input::TermRead;


const DEFAULT_PROMPT: &str = "$ ";

pub struct Editor<'s> {
    shell: &'s mut Shell,
}

impl<'s> Editor<'s> {
    pub fn new(shell: &'s mut Shell) -> Self {
        Self {
            shell
        }
    }

    pub fn run(&mut self) -> i32 {
        loop {
            if builtins::exit::exit_requested() {
                break;
            }

            write!(self.shell.stdout, "{}", DEFAULT_PROMPT);
            self.shell.stdout.flush();

            match self.shell.stdin.read_line() {
                Ok(Some(line)) => {
                    self.dispatch(line);
                }
                Ok(None) => {}
                Err(err) => {
                    println!("Error: {:?}", err);
                    builtins::exit::exit(Some(130));
                }
            }
        }

        *::exit_status()
    }

    fn dispatch(&mut self, line: String) {
        // Build a parser.
        let mut scanner = scanner::StringScanner::new(&line);
        let parser = parser::Parser::new(&mut scanner);

        // Parse the command line as a script.
        let expression = match parser.parse() {
            Ok(e) => e,
            Err(e) => {
                println!("error: {}\n    {}:{}:{}",
                         e.kind.description(),
                         self.shell.name(),
                         e.pos.line,
                         e.pos.column);
                return;
            }
        };

        execute::execute(&expression);
    }
}
