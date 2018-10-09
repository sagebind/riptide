//! The Riptide runtime.
use builtins;
use exceptions::Exception;
use modules;
use std::rc::Rc;
use stdlib;
use string::RString;
use syntax;
use syntax::ast::*;
use syntax::source::*;
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
            .module_loader(stdlib::stdlib_loader)
            .module_loader(modules::relative_loader)
            .module_loader(modules::system_loader)
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            module_loaders: Vec::new(),
            globals: table! {
                "args" => Value::ForeignFunction(builtins::args),
                "call" => Value::ForeignFunction(builtins::call),
                "catch" => Value::ForeignFunction(builtins::catch),
                "def" => Value::ForeignFunction(builtins::def),
                "list" => Value::ForeignFunction(builtins::list),
                "nil" => Value::ForeignFunction(builtins::nil),
                "require" => Value::ForeignFunction(builtins::require),
                "throw" => Value::ForeignFunction(builtins::throw),
                "typeof" => Value::ForeignFunction(builtins::type_of),
            },
        }
    }

    /// Register a module loader.
    pub fn module_loader(mut self, loader: impl modules::ModuleLoader + 'static) -> Self {
        self.module_loaders.push(Rc::new(loader));
        self
    }

    pub fn build(self) -> Runtime {
        let mut runtime = Runtime {
            module_loaders: self.module_loaders,
            globals: self.globals,
            module_registry: Table::default(),
            call_stack: Vec::new(),
            is_exiting: false,
            exit_code: 0,
        };

        runtime.init();

        runtime
    }
}

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    /// Registered module loaders, which are used to load modules the runtime has not seen before.
    module_loaders: Vec<Rc<modules::ModuleLoader>>,

    /// Table where global values are stored.
    globals: Table,

    /// Table of tables for module-level values.
    module_registry: Table,

    call_stack: Vec<CallFrame>,

    /// If application code inside the runtime requests the runtime to exit, this is set to true.
    is_exiting: bool,

    /// The runtime exit code to return after it shuts down.
    exit_code: i32,
}

/// Contains information about the current function call.
pub(crate) struct CallFrame {
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
        self.execute(None, include_str!("init.rip"))
            // This should never throw an exception.
            .unwrap();
    }

    pub fn load_module(&mut self, name: impl AsRef<str>) -> Result<Value, Exception> {
        let name = name.as_ref();

        if let Some(value) = self.module_registry.get(name) {
            return Ok(value);
        }

        debug!("module '{}' not defined, calling loader chain", name);

        for loader in self.module_loaders.clone() {
            match loader.load(self, name) {
                Ok(Value::Nil) => continue,
                Err(exception) => return Err(exception),
                Ok(value) => {
                    self.module_registry.set(name, value.clone());
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
        self.is_exiting
    }

    /// Request the runtime to exit with the given exit code.
    ///
    /// The runtime will exit gracefully.
    pub fn exit(&mut self, code: i32) {
        debug!("runtime exit requested with exit code {}", code);
        self.exit_code = code;
        self.is_exiting = true;
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
        let mut result = None;

        for part in &path.0 {
            match part {
                VariablePathPart::Ident(part) => {
                    result = match result.take() {
                        Some(Value::Table(table)) => table.get(part),
                        None => self.get(part),
                        _ => return None,
                    }
                }
            }
        }

        result
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
    pub(crate) fn current_frame(&self) -> &CallFrame {
        self.call_stack.last().unwrap()
    }

    /// Execute the given script within this runtime.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub fn execute(&mut self, module: Option<&str>, file: impl Into<SourceFile>) -> Result<Value, Exception> {
        let _module = module.and_then(|name| self.module_registry.get(name));

        let block = match syntax::parse(file) {
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
