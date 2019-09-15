//! The reactor functions as the runtime's core event loop, and is responsible
//! for keeping track of all I/O operations and signals.

use super::{ReadHandle, RegisterRead, RegisterWrite, WriteHandle};
use futures::{
    pin_mut,
    task::{ArcWake, AtomicWaker},
};
use mio::{unix::EventedFd, Events, PollOpt, Ready, Registration, SetReadiness, Token};
use signal_hook::iterator::Signals;
use slab::Slab;
use std::{
    fmt,
    fs::File,
    future::Future,
    io,
    io::Write,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

const WAKER_TOKEN: Token = Token(usize::max_value() - 1);
const SIGNAL_TOKEN: Token = Token(usize::max_value() - 2);

pub struct IoRegistration {
    read_waker: AtomicWaker,
    write_waker: AtomicWaker,
}

/// Single-threaded, I/O based executor. Only understands pipes.
pub struct Reactor {
    poll: mio::Poll,
    events: Events,
    signals: Signals,
    waker: Waker,
    _waker_registration: Registration,
    registrations: Slab<IoRegistration>,
    // wake_reader: PipeReader,
    // waker: Waker,
}

impl Reactor {
    pub(crate) fn new() -> io::Result<Self> {
        let poll = mio::Poll::new().unwrap();

        let (registration, set_readiness) = Registration::new2();
        let waker = MioWaker(set_readiness).into();
        poll.register(
            &registration,
            WAKER_TOKEN,
            Ready::readable(),
            PollOpt::edge(),
        )?;

        let signals = Signals::new(&[]).unwrap();
        poll.register(&signals, SIGNAL_TOKEN, Ready::all(), PollOpt::level())?;

        Ok(Self {
            poll,
            events: Events::with_capacity(1024),
            signals,
            waker,
            _waker_registration: registration,
            registrations: Slab::new(),
            // wake_reader,
            // waker: Arc::new(PipeWaker(wake_writer)).into_waker(),
        })
    }

    /// Get this reactor's task context.
    pub(crate) fn task_context(&self) -> Context<'_> {
        Context::from_waker(&self.waker)
    }

    pub(crate) fn register_read(
        &mut self,
        handle: &mut impl RegisterRead,
        ready: Ready,
    ) -> io::Result<()> {
        let registration = IoRegistration {
            read_waker: AtomicWaker::default(),
            write_waker: AtomicWaker::default(),
        };

        let token = self.registrations.insert(registration);
        self.poll.register(
            &EventedFd(&handle.as_raw_fd()),
            Token(token),
            ready,
            PollOpt::level(),
        )?;

        let read_handle = ReadHandle::default();
        handle.init_read_handle(read_handle);

        Ok(())
    }

    fn poll(&mut self) -> io::Result<()> {
        self.poll.poll(&mut self.events, None)?;
        Ok(())
    }

    fn dispatch(&mut self) {
        for event in &self.events {
            match event.token() {
                WAKER_TOKEN => {
                    log::debug!("woke by waker");
                }

                SIGNAL_TOKEN => {
                    for signal in self.signals.pending() {
                        log::debug!("received signal: {}", signal);
                    }
                }

                Token(token) => {
                    if let Some(registration) = self.registrations.get(token) {
                        if event.readiness().is_readable() {
                            registration.read_waker.wake();
                        }

                        if event.readiness().is_writable() {
                            registration.write_waker.wake();
                        }
                    }
                }
            }
        }
    }

    pub fn run_until<F: Future>(&mut self, future: F) -> <F as Future>::Output {
        pin_mut!(future);

        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut self.task_context()) {
                return output;
            }

            self.poll().unwrap();
            self.dispatch();
        }
    }
}

struct MioWaker(SetReadiness);

impl From<MioWaker> for Waker {
    fn from(waker: MioWaker) -> Waker {
        futures::task::waker(Arc::new(waker))
    }
}

impl ArcWake for MioWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.0.set_readiness(Ready::readable()).ok();
    }
}
