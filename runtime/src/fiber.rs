use crate::{
    eval,
    exceptions::Exception,
    io::{IoContext, Input, Output},
    modules::{ModuleIndex, NativeModule},
    scope::Scope,
    string::RipString,
    syntax::source::SourceFile,
    table,
    table::Table,
    value::Value,
};
use gc::Gc;
use std::{rc::Rc, sync::atomic::{AtomicUsize, Ordering}};

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
pub struct Fiber {
    /// A short identifier for this fiber, for diagnostic purposes.
    pid: usize,

    module_index: Rc<ModuleIndex>,

    /// Table where global values are stored that are not on the stack.
    globals: Table,

    /// Default global values for context variables. This holds the values of
    /// context variables that have not been set by any scope.
    cvar_globals: Table,

    /// Call stack of functions being executed by this fiber.
    pub(crate) stack: Vec<Gc<Scope>>,

    /// Standard I/O streams for this fiber.
    pub(crate) io: IoContext,
}

impl Fiber {
    /// Create a new fiber with the given I/O context.
    pub(crate) fn new(io_cx: IoContext) -> Self {
        let fiber = Self {
            pid: next_pid(),
            module_index: Rc::new(ModuleIndex::default()),
            globals: Default::default(),
            cvar_globals: Default::default(),
            stack: Vec::new(),
            io: io_cx,
        };

        log::debug!("root fiber {} created", fiber.pid);

        match std::env::current_dir() {
            Ok(cwd) => {
                fiber.cvar_globals.set("cwd", cwd);
            }
            Err(e) => {
                log::warn!("failed to set initial cwd: {}", e);
            }
        }

        fiber
    }

    /// Get the table that holds all global variables.
    pub fn globals(&self) -> &Table {
        &self.globals
    }

    /// Get a handle to this fiber's standard input stream.
    pub fn stdin(&mut self) -> &mut dyn Input {
        self.io.stdin()
    }

    /// Get a handle to this fiber's standard output stream.
    pub fn stdout(&mut self) -> &mut dyn Output {
        self.io.stdout()
    }

    /// Get a handle to this fiber's standard error stream.
    pub fn stderr(&mut self) -> &mut dyn Output {
        self.io.stderr()
    }

    /// Create a new fiber with the exact same stack and context as this one.
    pub fn fork(&self) -> Self {
        let fork = Self {
            pid: next_pid(),
            module_index: self.module_index.clone(),
            globals: self.globals.clone(),
            cvar_globals: self.cvar_globals.clone(),
            stack: self.stack.clone(),
            io: self.io.try_clone().unwrap(),
        };

        log::debug!("fiber {} forked from fiber {}", fork.pid, self.pid);

        fork
    }

    /// Get the fiber's current working directory.
    pub fn current_dir(&self) -> Value {
        // The working dir is just implemented as the `@cwd` context variable.
        self.get_cvar("cwd")
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
        let closure = eval::compile(self, file)?;

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

        self.cvar_globals.get(name)
    }

    /// Force the garbage collector to run now.
    pub fn collect_garbage(&mut self) {
        gc::force_collect();
    }

    /// Get the current executing scope.
    pub(crate) fn current_scope(&self) -> Option<&Gc<Scope>> {
        self.stack.last()
    }

    /// Get a backtrace-like view of the stack.
    pub(crate) fn backtrace(&self) -> impl Iterator<Item = &Gc<Scope>> {
        self.stack.iter().rev()
    }

    /// Register a module implemented in native code.
    pub fn register_native_module<N, M>(&self, name: N, module: M)
    where
        N: Into<String>,
        M: NativeModule + 'static,
    {
        self.module_index.register_native_module(name, module);
    }

    pub(crate) async fn load_module(&mut self, name: &str) -> Result<Value, Exception> {
        self.module_index.clone().load(self, name).await
    }
}

impl Drop for Fiber {
    fn drop(&mut self) {
        self.collect_garbage();
        log::debug!("fiber {} dropped", self.pid);
    }
}
