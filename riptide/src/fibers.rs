use fd;
use std::os::unix::io::FromRawFd;

pub type FID = usize;

pub struct Fiber {
    /// Unique fiber ID.
    id: FID,

    /// Standard input stream.
    stdin: Option<fd::ReadPipe>,

    /// Standard output stream.
    stdout: Option<fd::WritePipe>,

    /// Standard error stream.
    stderr: Option<fd::WritePipe>,

    /// List of child fibers.
    children: Vec<Fiber>,

    /// List of processes owned by this fiber.
    processes: Vec<()>,
}

pub enum FiberState {
    Paused,
    Running,
    Terminated,
}

impl Fiber {
    /// Get the root fiber for this process.
    pub fn root() -> Fiber {
        unsafe {
            Fiber {
                id: 1,
                stdin: Some(fd::ReadPipe::from_raw_fd(0)),
                stdout: Some(fd::WritePipe::from_raw_fd(1)),
                stderr: Some(fd::WritePipe::from_raw_fd(2)),
                children: Vec::new(),
                processes: Vec::new(),
            }
        }
    }

    pub fn stdin(&mut self) -> Option<&mut fd::ReadPipe> {
        self.stdin.as_mut()
    }

    /// Create a new fiber, with this fiber as its parent.
    ///
    /// The new fiber inherits the same file descriptors as its parent.
    ///
    /// This is a cheap (though not free) operation.
    pub fn fork(&self) -> Fiber {
        Fiber {
            id: 2,
            stdin: self.stdin.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            children: Vec::new(),
            processes: Vec::new(),
        }
    }
}

pub struct Scheduler {
    root: Fiber,
    current: Option<FID>,
}

impl Scheduler {
    /// This function is **not** thread-safe!
    pub fn global() -> &'static mut Scheduler {
        static mut GLOBAL: Option<Scheduler> = None;

        unsafe {
            if GLOBAL.is_none() {
                GLOBAL = Some(Scheduler {
                    root: Fiber::root(),
                    current: None,
                });
            }

            GLOBAL.as_mut().unwrap()
        }
    }

    pub fn run(&mut self) {}
}

pub enum CallResult {
    Ok,
    Err,
    Yield,
}
