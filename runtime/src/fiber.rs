use super::{
    eval, exceptions::Exception, scope::Scope, string::RipString,
    table::Table, value::Value,
};
use crate::{
    io::{IoContext, PipeReader, PipeWriter},
    syntax::source::SourceFile,
};
use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

/// This is the name of the hidden global variable that exit code requests are
/// stored in.
static EXIT_CODE_GLOBAL: &str = "__exit_code";

fn next_pid() -> usize {
    static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

    NEXT_PID.fetch_add(1, Ordering::SeqCst)
}

/// A fiber is an internal concept of a "thread of execution" which allows
/// multiple call stacks to be tracked when executing parallel pipelines.
///
/// Fibers are scheduled co-operatively on a single main thread.
#[derive(Debug)]
pub struct Fiber {
    /// A short identifier for this fiber, for diagnostic purposes.
    pid: usize,

    /// Table where global values are stored that are not on the stack.
    globals: Table,

    /// Call stack of functions being executed by this fiber.
    pub(crate) stack: Vec<Rc<Scope>>,

    /// Standard I/O streams for this fiber.
    pub(crate) io: IoContext,
}

impl Fiber {
    /// Create a new fiber with the given I/O context.
    pub(crate) fn new(io_cx: IoContext) -> Self {
        let fiber = Self {
            pid: next_pid(),
            globals: Default::default(),
            stack: Vec::new(),
            io: io_cx,
        };

        log::debug!("root fiber {} created", fiber.pid);

        fiber
    }

    /// Get the table that holds all global variables.
    pub fn globals(&self) -> &Table {
        &self.globals
    }

    /// Get a handle to this fiber's standard input stream.
    pub fn stdin(&mut self) -> &mut PipeReader {
        &mut self.io.stdin
    }

    /// Get a handle to this fiber's standard output stream.
    pub fn stdout(&mut self) -> &mut PipeWriter {
        &mut self.io.stdout
    }

    /// Get a handle to this fiber's standard error stream.
    pub fn stderr(&mut self) -> &mut PipeWriter {
        &mut self.io.stderr
    }

    /// Create a new fiber with the exact same stack and context as this one.
    pub fn fork(&self) -> Self {
        let fork = Self {
            pid: next_pid(),
            globals: self.globals.clone(),
            stack: self.stack.clone(),
            io: self.io.try_clone().unwrap(),
        };

        log::debug!("fiber {} forked from fiber {}", fork.pid, self.pid);

        fork
    }

    /// Get the fiber's current working directory.
    pub fn current_dir(&self) -> Value {
        // First check the `@cwd` context variable.
        let mut cwd = self.get_cvar("cwd");

        // If not set, check the process-wide (default) working directory.
        if cwd.is_nil() {
            if let Ok(path) = std::env::current_dir() {
                cwd = path.into();
            }
        }

        cwd
    }

    /// Get the current exit code for the runtime. If no exit has been
    /// requested, then `None` will be returned.
    ///
    /// Note that "exiting" is a global activity that involves all related
    /// fibers, and that the value returned here could have been set by a
    /// different fiber.
    pub fn exit_code(&self) -> Option<i32> {
        match self.globals.get(EXIT_CODE_GLOBAL) {
            Value::Number(num) => Some(num as i32),
            _ => None,
        }
    }

    /// Request the runtime to exit with the given exit code. The request is
    /// global and visible by all related fibers.
    pub fn exit(&self, code: i32) {
        log::debug!("exit requested with code {}", code);

        // Set exit code if absent, or upgrade a zero exit code to a nonzero
        // one.
        if let None | Some(0) = self.exit_code() {
            self.globals.set(EXIT_CODE_GLOBAL, code as f64);
        }
    }

    /// Execute the given script within this runtime.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute(
        &mut self,
        module: Option<&str>,
        file: impl Into<SourceFile>,
    ) -> Result<Value, Exception> {
        self.execute_in_scope(module, file, table!()).await
    }

    /// Execute the given script using the given scope.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute_in_scope(
        &mut self,
        _module: Option<&str>,
        file: impl Into<SourceFile>,
        scope: Table,
    ) -> Result<Value, Exception> {
        let closure = eval::compile(self, file, None)?;

        eval::invoke_closure(self, &closure, vec![], scope, Default::default()).await
    }

    /// Invoke the given value as a function with the given arguments.
    pub async fn invoke(&mut self, value: &Value, args: &[Value]) -> Result<Value, Exception> {
        eval::invoke(self, value, args.to_vec()).await
    }

    /// Lookup a normal variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        if let Some(scope) = self.stack.last() {
            match scope.get(name) {
                Value::Nil => {}
                value => return value,
            }
        }

        self.globals.get(name)
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        self.current_scope().unwrap().set(name, value);
    }

    /// Get the current value of a context variable.
    pub fn get_cvar(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        // Cvars are just implemented as dynamic scoping; backtrack up through
        // the stack and look for an appropriate cvar.
        for scope in self.stack.iter().rev() {
            match scope.cvars.get(name) {
                Value::Nil => {}
                value => return value,
            }
        }

        Value::Nil
    }

    /// Get the current executing scope.
    pub(crate) fn current_scope(&self) -> Option<&Rc<Scope>> {
        self.stack.last()
    }

    /// Get a backtrace-like view of the stack.
    pub(crate) fn backtrace(&self) -> impl Iterator<Item = &Rc<Scope>> {
        self.stack.iter().rev()
    }

    /// Get a module scope table by the module's name. If the module table does
    /// not already exist, it will be created.
    pub(crate) fn get_module_by_name(&self, name: &str) -> Table {
        let loaded = self.globals.get("modules").get("loaded");

        if loaded.get(name).is_nil() {
            loaded.as_table().unwrap().set(name, table!());
        }

        loaded.get(name).as_table().unwrap().clone()
    }
}

impl Drop for Fiber {
    fn drop(&mut self) {
        log::debug!("fiber {} dropped", self.pid);
    }
}
