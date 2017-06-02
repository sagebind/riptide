use nix::*;
use parser::Expression;
use std::fs::File;
use std::os::unix::prelude::*;


/// Form a pipeline between a series of calls and execute them in parallel.
pub fn main(args: &[Expression]) {
}

/// Create a new I/O pipe.
fn pipe() -> (File, File) {
    let fds = unistd::pipe().unwrap();

    unsafe {
        (File::from_raw_fd(fds.0), File::from_raw_fd(fds.1))
    }
}
