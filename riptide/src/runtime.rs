use ast::Expr;
use fd;
use std::os::unix::io::FromRawFd;
use value::Value;

pub type FID = usize;

/// Single fiber of execution. Contains both the interpeter stack state for the
/// fiber as well as any contextual handles.
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

    /// Call stack.
    call_stack: Vec<CallFrame>,

    /// Stack of expressions to be evaluated.
    expr_stack: Vec<Expr>,

    /// Arguments to be passed to the next function call.
    arg_stack: Vec<Value>,
}

struct CallFrame {
    /// Real args passed to the current function.
    args: Vec<Value>,

    pc: usize,
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
                call_stack: Vec::new(),
                arg_stack: Vec::new(),
                expr_stack: Vec::new(),
            }
        }
    }

    pub fn stdin(&mut self) -> Option<&mut fd::ReadPipe> {
        self.stdin.as_mut()
    }
}

impl Clone for Fiber {
    /// Create a new fiber, with this fiber as its parent.
    ///
    /// The new fiber inherits the same file descriptors as its parent.
    ///
    /// This is a cheap (though not free) operation.
    fn clone(&self) -> Self {
        Fiber {
            id: 2,
            stdin: self.stdin.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            children: Vec::new(),
            processes: Vec::new(),
            call_stack: Vec::new(),
            arg_stack: Vec::new(),
            expr_stack: Vec::new(),
        }
    }
}

/// Executes program code and holds its state.
pub struct Runtime {
    /// The currently running fiber, if any.
    current: Option<FID>,

    reg_in: Vec<Expr>,
    reg_out: Vec<Value>,
    pc: usize,
}

impl Runtime {
    pub fn execute(&mut self, program: Expr) {
        self.reg_in.clear();
        self.reg_in.push(program);

        self.resume();
    }

    fn resume(&mut self) {
        loop {
            match self.reg_in.pop() {
                // Sacrebleu, a function call! Surprise, surprise.
                Some(Expr::Call(call)) => {
                    self.reg_in.extend(call.args);
                },

                Some(_) => {},

                None => break,
            }
        }
    }

    /// Execute the current register values as a function call.
    fn do_call(&mut self) {}
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
