use crate::io::{
    IoContext,
    PipeReader,
    PipeWriter,
};
use super::{
    eval,
    exceptions::Exception,
    foreign::ForeignFn,
    scope::Scope,
    string::RipString,
    syntax::source::SourceFile,
    table::Table,
    value::Value,
};
use std::rc::Rc;

static EXIT_CODE_GLOBAL: &str = "__exit_code";

/// A fiber is an internal concept of a "thread of execution" which allows
/// multiple call stacks to be tracked when executing parallel pipelines.
///
/// Fibers are scheduled co-operatively on a single main thread.
pub struct Fiber {
    /// Table where global values are stored that are not on the stack.
    pub(crate) globals: Table,

    /// Call stack of functions being executed by this fiber.
    pub(crate) stack: Vec<Rc<Scope>>,

    /// Standard I/O streams for this fiber.
    pub(crate) io: IoContext,
}

impl Fiber {
    pub(crate) fn new(io_cx: IoContext) -> Self {
        Self {
            globals: Default::default(),
            stack: Vec::new(),
            io: io_cx,
        }
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

    pub fn fork(&self) -> Self {
        Self {
            globals: self.globals.clone(),
            stack: self.stack.clone(),
            io: self.io.try_clone().unwrap(),
        }
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
        self.globals.set(EXIT_CODE_GLOBAL, code as f64);
    }

    /// Register a module loader.
    pub(crate) fn register_module_loader(&self, loader: impl Into<ForeignFn>) {
        let modules = self.globals.get("modules").as_table().unwrap();
        modules.set("loaders", modules.get("loaders").append(loader.into()).unwrap());
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

    /// Execute the given script within this runtime.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute(&mut self, module: Option<&str>, file: impl Into<SourceFile>) -> Result<Value, Exception> {
        self.execute_in_scope(module, file, table!()).await
    }

    /// Execute the given script using the given scope.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute_in_scope(&mut self, _module: Option<&str>, file: impl Into<SourceFile>, scope: Table) -> Result<Value, Exception> {
        let closure = eval::compile(self, file, Some(scope))?;

        eval::invoke_closure(self, &closure, &[]).await
    }

    /// Invoke the given value as a function with the given arguments.
    pub async fn invoke(&mut self, value: &Value, args: &[Value]) -> Result<Value, Exception> {
        eval::invoke(self, value, args).await
    }

    /// Lookup a variable name in the current scope.
    #[deprecated]
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        if let Some(scope) = self.stack.last() {
            match scope.get(name) {
                Value::Nil => {},
                value => return value,
            }
        }

        self.globals.get(name)
    }

    /// Set a variable value in the current scope.
    #[deprecated]
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        self.current_scope().unwrap().set(name, value);
    }

    #[deprecated]
    pub(crate) fn set_parent(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        if self.stack.len() >= 2 {
            self.stack[self.stack.len() - 2].set(name, value);
        }
    }
}
