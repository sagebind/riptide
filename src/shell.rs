use builtins;
use parser;
use rustyline;
use rustyline::error::ReadlineError;
use scanner;


const DEFAULT_PROMPT: &str = "$ ";


pub struct Shell {
    editor: rustyline::Editor<()>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            editor: rustyline::Editor::new()
        }
    }

    pub fn run(&mut self) -> i32 {
        loop {
            if builtins::exit::exit_requested() {
                break;
            }

            match self.editor.readline(DEFAULT_PROMPT) {
                Ok(line) => {
                    self.dispatch(line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    builtins::exit::exit(None);
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    builtins::exit::exit(Some(30));
                }
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
                println!("{}", e);
                return;
            },
        };

        println!("{}", expression);

        self.editor.add_history_entry(line.as_ref());
    }
}
