//! The reactor functions as the runtime's core event loop, and is responsible
//! for keeping track of all I/O operations and signals.

use futures::task::ArcWake;
use mio::{Events, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use signal_hook::iterator::Signals;
use std::{
    future::Future,
    io,
    io::Write,
    sync::Arc,
    task::{Context, Waker},
};

const WAKER_TOKEN: Token = Token(0);
const SIGNAL_TOKEN: Token = Token(1);

/// Single-threaded, I/O based executor. Only understands pipes.
pub struct Reactor {
    poll: Poll,
    events: Events,
    signals: Signals,
    registration: Registration,
    waker: Waker,
    // wake_reader: PipeReader,
    // waker: Waker,
}

impl Reactor {
    pub(crate) fn new() -> io::Result<Self> {
        let poll = Poll::new().unwrap();

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
            registration,
            waker,
            // wake_reader,
            // waker: Arc::new(PipeWaker(wake_writer)).into_waker(),
        })
    }

    /// Get this reactor's task context.
    pub(crate) fn task_context(&self) -> Context<'_> {
        Context::from_waker(&self.waker)
    }

    fn wait(&mut self) {
        self.poll.poll(&mut self.events, None).unwrap();
    }

    fn dispatch(&mut self) {
        for event in &self.events {
            if event.token() == SIGNAL_TOKEN {
                for signal in self.signals.pending() {
                    log::debug!("received signal: {}", signal);
                }
            }
        }
    }

    pub fn run_until<F: Future>(&mut self, future: F) -> <F as Future>::Output {
        loop {
            self.wait();
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
