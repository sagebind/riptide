//! The Riptide runtime.
use builtins;
use riptide_syntax;
use riptide_syntax::filemap::FileMap;
use riptide_syntax::ast::*;
use value::Value;
use value::table::Table;

pub type ForeignFunction = fn(&mut Runtime, &[Value]) -> Result<Value, Exception>;

pub type ModuleLoaderFn = fn(&str) -> Result<Value, Exception>;

#[derive(Clone, Debug)]
pub struct Exception(pub Value);

pub struct RuntimeBuilder {
    module_resolver: Option<ModuleLoaderFn>,
    globals: Table,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            module_resolver: None,
            globals: Table::new(),
        }
    }
}

impl RuntimeBuilder {
    pub fn preload(mut self, module: &str) -> Self {
        self
    }

    pub fn with_stdlib(mut self) -> Self {
        self.globals.set("exit", Value::ForeignFunction(builtins::exit));
        self.globals.set("print", Value::ForeignFunction(builtins::print));
        self.globals.set("println", Value::ForeignFunction(builtins::println));
        self.globals.set("typeof", Value::ForeignFunction(builtins::type_of));
        self.globals.set("list", Value::ForeignFunction(builtins::list));
        self.globals.set("nil", Value::ForeignFunction(builtins::nil));
        self.globals.set("throw", Value::ForeignFunction(builtins::throw));
        self.globals.set("catch", Value::ForeignFunction(builtins::catch));
        self.globals.set("args", Value::ForeignFunction(builtins::args));

        self
    }

    pub fn build(self) -> Runtime {
        Runtime {
            module_resolver: self.module_resolver,
            module_cache: Table::new(),
            globals: self.globals,
            call_stack: Vec::new(),
            exit_code: 0,
            exit_requested: false,
        }
    }
}

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    /// Module loaders to use when requiring a module.
    module_resolver: Option<ModuleLoaderFn>,

    /// Cache of modules already required.
    module_cache: Table,

    /// Holds global variable bindings.
    globals: Table,

    /// Function call stack containing call frames.
    call_stack: Vec<CallFrame>,

    exit_code: i32,

    exit_requested: bool,
}

/// Contains information about the current function call.
pub struct CallFrame {
    pub args: Vec<Value>,
    pub bindings: Table,
}

impl Runtime {
    pub fn load_module(&mut self, name: &str) -> Result<Value, Exception> {
        if let Some(value) = self.module_cache.get(name) {
            return Ok(value);
        }

        if let Some(loader) = self.module_resolver {
            match (loader)(name) {
                Ok(value) => {
                    self.module_cache.set(name, value.clone());
                    return Ok(value);
                },
                Err(exception) => return Err(exception),
            }
        }

        Err(Exception(Value::from("module not found")))
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn request_exit(&mut self, code: i32) {
        self.exit_code = code;
        self.exit_requested = true;
    }

    pub fn get_global(&self, name: &str) -> Option<Value> {
        self.globals.get(name)
    }

    pub fn set_global<V: Into<Value>>(&mut self, name: &str, value: V) {
        self.globals.set(name, value);
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: &str) -> Option<Value> {
        for frame in self.call_stack.iter().rev() {
            if let Some(value) = frame.bindings.get(name) {
                return Some(value);
            }
        }

        self.get_global(name)
    }

    /// Get a reference to the current call stack frame.
    pub fn current_frame(&self) -> &CallFrame {
        self.call_stack.last().unwrap()
    }

    /// Execute the given script within this runtime context.
    pub fn execute(&mut self, script: &str) -> Result<Value, Exception> {
        self.execute_filemap(FileMap::buffer(None, script))
    }

    fn execute_filemap(&mut self, filemap: FileMap) -> Result<Value, Exception> {
        let block = match riptide_syntax::parse(filemap) {
            Ok(block) => block,
            Err(e) => return Err(Exception(Value::from(format!("error parsing: {}", e.message)))),
        };

        self.invoke_block(&block, &[])
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
        for statement in block.statements.iter().rev() {
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
            _ => Err(Exception(Value::from(format!("cannot execute {:?} as a function", function)))),
        }
    }

    fn evaluate_expr(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::Number(number) => Ok(Value::Number(number)),
            Expr::String(string) => Ok(Value::from(string)),
            // TODO: Handle expands
            Expr::ExpandableString(string) => Ok(Value::from(string)),
            Expr::Block(block) => Ok(Value::from(block)),
            Expr::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline),
        }
    }
}
