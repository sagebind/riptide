use parser::Expression;
use std::io;
use termion;


/// Form a pipeline between a series of calls and execute them in parallel.
pub fn main(args: &[Expression]) {
    if termion::is_tty(&io::stdin()) {}
}
