//! The Riptide runtime.
use builtins;
use exceptions::Exception;
use modules;
use riptide_syntax;
use riptide_syntax::ast::*;
use riptide_syntax::source::*;
use std::path::Path;
use std::rc::Rc;
use string::RString;
use table::Table;
use value::Value;

pub type ForeignFunction = fn(&mut Runtime, &[Value]) -> Result<Value, Exception>;

/// Configure a runtime.
pub struct RuntimeBuilder {
    module_loaders: Vec<Rc<modules::ModuleLoader>>,
    globals: Table,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
            .module_loader(modules::relative_loader)
            .module_loader(modules::system_loader)
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            module_loaders: Vec::new(),
            globals: Table::new(),
        }
    }

    /// Register a module loader.
    pub fn module_loader<T>(mut self, loader: T) -> Self where T: modules::ModuleLoader + 'static {
        self.module_loaders.push(Rc::new(loader));
        self
    }

    pub fn build(self) -> Runtime {
        let mut runtime = Runtime {
            module_loaders: self.module_loaders,
            module_cache: Table::new(),
            globals: self.globals,
            call_stack: Vec::new(),
            exit_code: 0,
            exit_requested: false,
        };

        runtime.init();

        runtime
    }
}

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    module_loaders: Vec<Rc<modules::ModuleLoader>>,
    module_cache: Table,
    globals: Table,
    call_stack: Vec<CallFrame>,
    exit_code: i32,
    exit_requested: bool,
}

/// Contains information about the current function call.
pub struct CallFrame {
    pub args: Vec<Value>,
    pub bindings: Table,
}

impl Default for Runtime {
    fn default() -> Self {
        RuntimeBuilder::default().build()
    }
}

impl Runtime {
    /// Initialize the runtime environment.
    fn init(&mut self) {
        self.globals.set("args", Value::ForeignFunction(builtins::args));
        self.globals.set("call", Value::ForeignFunction(builtins::call));
        self.globals.set("catch", Value::ForeignFunction(builtins::catch));
        self.globals.set("def", Value::ForeignFunction(builtins::def));
        self.globals.set("exit", Value::ForeignFunction(builtins::exit));
        self.globals.set("list", Value::ForeignFunction(builtins::list));
        self.globals.set("nil", Value::ForeignFunction(builtins::nil));
        self.globals.set("print", Value::ForeignFunction(builtins::print));
        self.globals.set("println", Value::ForeignFunction(builtins::println));
        self.globals.set("require", Value::ForeignFunction(builtins::require));
        self.globals.set("throw", Value::ForeignFunction(builtins::throw));
        self.globals.set("typeof", Value::ForeignFunction(builtins::type_of));

        self.execute(include_str!("init.rip"))
            // This should never throw an exception.
            .unwrap();
    }

    pub fn load_module(&mut self, name: impl AsRef<str>) -> Result<Value, Exception> {
        let name = name.as_ref();

        if let Some(value) = self.module_cache.get(name) {
            return Ok(value);
        }

        debug!("module '{}' not loaded, calling loader chain", name);

        for loader in self.module_loaders.clone() {
            match loader.load(self, name) {
                Ok(Value::Nil) => continue,
                Err(exception) => return Err(exception),
                Ok(value) => {
                    self.module_cache.set(name, value.clone());
                    return Ok(value);
                },
            }
        }

        Err(Exception::from("module not found"))
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn request_exit(&mut self, code: i32) {
        info!("runtime exit requested with exit code {}", code);
        self.exit_code = code;
        self.exit_requested = true;
    }

    pub fn get_global(&self, name: impl AsRef<[u8]>) -> Option<Value> {
        self.globals.get(name)
    }

    pub fn set_global(&mut self, name: impl Into<RString>, value: impl Into<Value>) {
        self.globals.set(name, value);
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Option<Value> {
        let name = name.as_ref();

        for frame in self.call_stack.iter().rev() {
            if let Some(value) = frame.bindings.get(name) {
                return Some(value);
            }
        }

        self.get_global(name)
    }

    fn get_path(&self, path: &VariablePath) -> Option<Value> {
        self.get(path.to_string())
    }

    /// Set a variable value in the current scope.
    pub fn set(&mut self, name: impl Into<RString>, value: impl Into<Value>) {
        if let Some(ref mut frame) = self.call_stack.last_mut() {
            frame.bindings.set(name, value);
            return;
        }

        warn!("set called with an empty call stack");
        self.set_global(name, value);
    }

    /// Get a reference to the current call stack frame.
    pub fn current_frame(&self) -> &CallFrame {
        self.call_stack.last().unwrap()
    }

    /// Execute the given script within this runtime context.
    pub fn execute(&mut self, script: &str) -> Result<Value, Exception> {
        self.execute_filemap(SourceFile::from(script))
    }

    pub fn execute_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Value, Exception> {
        self.execute_filemap(SourceFile::open(path)?)
    }

    fn execute_filemap(&mut self, file: SourceFile) -> Result<Value, Exception> {
        let block = match riptide_syntax::parse(file) {
            Ok(block) => block,
            Err(e) => return Err(Exception::from(format!("error parsing: {}", e))),
        };

        self.invoke_block(&block, &[])
    }

    /// Invoke the given value as a function with the given arguments.
    pub fn invoke(&mut self, value: &Value, args: &[Value]) -> Result<Value, Exception> {
        match value {
            Value::Block(block) => self.invoke_block(block, args),
            Value::ForeignFunction(function) => (function)(self, args),
            _ => Err(format!("cannot invoke {:?} as a function", value))?,
        }
    }

    /// Invoke a block with an array of arguments.
    pub fn invoke_block(&mut self, block: &Block, args: &[Value]) -> Result<Value, Exception> {
        // Set up a new stack frame.
        self.call_stack.push(CallFrame {
            args: args.to_vec(),
            bindings: Table::new(),
        });

        let mut last_return_value = Value::Nil;

        // Evaluate each statement in order.
        for statement in block.statements.iter() {
            match self.evaluate_pipeline(&statement) {
                Ok(return_value) => last_return_value = return_value,
                Err(exception) => {
                    // Exception thrown; abort and unwind stack.
                    self.call_stack.pop();
                    return Err(exception);
                },
            }
        }

        self.call_stack.pop();

        Ok(last_return_value)
    }

    fn evaluate_pipeline(&mut self, pipeline: &Pipeline) -> Result<Value, Exception> {
        // If there's only one call in the pipeline, we don't need to fork and can just execute the function by itself.
        if pipeline.items.len() == 1 {
            self.evaluate_call(pipeline.items[0].clone())
        } else {
            Ok(Value::Nil)
        }
    }

    fn evaluate_call(&mut self, call: Call) -> Result<Value, Exception> {
        let mut function = self.evaluate_expr(*call.function)?;

        let mut args = Vec::with_capacity(call.args.len());
        for expr in call.args {
            args.push(self.evaluate_expr(expr)?);
        }

        // If the function is a string, resolve binding names first before we try to eval the item as a function.
        if let Some(value) = function.as_string().and_then(|name| self.get(name)) {
            function = value;
        }

        // Execute the function.
        match function {
            Value::Block(block) => self.invoke_block(&block, &args),
            Value::ForeignFunction(f) => {
                f(self, &args)
            },
            _ => Err(Exception::from(format!("cannot execute {:?} as a function", function))),
        }
    }

    fn evaluate_expr(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::Number(number) => Ok(Value::Number(number)),
            Expr::String(string) => Ok(Value::from(string)),
            // TODO: Handle expands
            Expr::InterpolatedString(_) => Ok(Value::Nil),
            Expr::Substitution(substitution) => self.evaluate_substitution(substitution),
            Expr::Block(block) => Ok(Value::from(block)),
            Expr::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline),
        }
    }

    fn evaluate_substitution(&mut self, substitution: Substitution) -> Result<Value, Exception> {
        match substitution {
            Substitution::Variable(path) => match self.get_path(&path) {
                Some(name) => Ok(Value::from(name)),
                None => Err(Exception::from("undefined variable")),
            },
            Substitution::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline),
            _ => unimplemented!(),
        }
    }
}
