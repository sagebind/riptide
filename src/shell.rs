use builtins;
use execute;
use parser;
use termion;
use termion::input::TermRead;
use scanner;
use std::io::{self, Read, Write};


const DEFAULT_PROMPT: &str = "$ ";


pub struct Shell {
    filename: String,
    stdin: Box<Read>,
    stdout: Box<Write>,
    stderr: Box<Write>,
}

impl Default for Shell {
    fn default() -> Self {
        Shell::new("<stdin>", io::stdin(), io::stdout(), io::stderr())
    }
}

impl Shell {
    pub fn new<S, I, O, E>(name: S, stdin: I, stdout: O, stderr: E) -> Self
        where S: Into<String>,
              I: Read + 'static,
              O: Write + 'static,
              E: Write + 'static
    {
        Self {
            filename: name.into(),
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
            stderr: Box::new(stderr),
        }
    }

    pub fn run(&mut self) -> i32 {
        loop {
            if builtins::exit::exit_requested() {
                break;
            }

            print!("{}", DEFAULT_PROMPT);
            self.stdout.flush();

            match self.stdin.read_line() {
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

        builtins::exit::exit_code()
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
                         self.filename,
                         e.pos.line,
                         e.pos.column);
                return;
            }
        };

        execute::execute(&expression);
    }
}

