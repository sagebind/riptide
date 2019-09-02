//! Asynchronous I/O registration interfaces.
//!
//! This module defines the interface used by pipes and other file descriptors
//! for communicating with the reactor.

use futures::task::AtomicWaker;
use std::os::unix::io::AsRawFd;

#[derive(Default)]
pub(crate) struct ReadHandle {
    pub(crate) waker: AtomicWaker,
}

pub(crate) trait RegisterRead: AsRawFd {
    fn init_read_handle(&mut self, handle: ReadHandle);
}

#[derive(Default)]
pub(crate) struct WriteHandle {
    pub(crate) waker: AtomicWaker,
}

pub(crate) trait RegisterWrite: AsRawFd {
    fn init_write_handle(&mut self, handle: WriteHandle);
}
