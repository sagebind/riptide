use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::{
    io,
    os::unix::io::{AsRawFd, FromRawFd},
};

pub(super) fn dup<T: AsRawFd, U: FromRawFd>(fd: T) -> io::Result<U> {
    nix::unistd::dup(fd.as_raw_fd())
        .map_err(nix_err)
        .map(|fd| unsafe { U::from_raw_fd(fd) })
}

pub(super) fn set_nonblocking(file: &mut impl AsRawFd, nonblocking: bool) -> io::Result<()> {
    let mut flags = fcntl(file.as_raw_fd(), FcntlArg::F_GETFL)
        .map(OFlag::from_bits)
        .map_err(nix_err)?
        .unwrap_or(OFlag::empty());

    flags.set(OFlag::O_NONBLOCK, nonblocking);

    fcntl(file.as_raw_fd(), FcntlArg::F_SETFL(flags))
        .map_err(nix_err)
        .map(|_| ())
}

fn nix_err(error: nix::Error) -> io::Error {
    if let nix::Error::Sys(err_no) = error {
        io::Error::from(err_no)
    } else {
        io::Error::new(io::ErrorKind::Other, error)
    }
}
