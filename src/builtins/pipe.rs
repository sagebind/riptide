use nix::*;
use std::fs::File;
use std::os::unix::prelude::*;


pub fn pipe() -> (File, File) {
    let fds = unistd::pipe().unwrap();

    unsafe {
        (File::from_raw_fd(fds.0), File::from_raw_fd(fds.1))
    }
}
