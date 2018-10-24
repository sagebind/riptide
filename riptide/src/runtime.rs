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
use value::*;

/// A native function that can be called by managed code.
pub type ForeignFunction = fn(&mut Runtime, &[Value]) -> Result<Value, Exception>;

/// Configure a runtime.
pub struct RuntimeBuilder {
    module_loaders: Vec<Value>,
    globals: Table,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
            .module_loader(modules::relative_loader)
            .module_loader(modules::system_loader)
            .with_stdlib()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            module_loaders: Vec::new(),
            globals: table! {
                "require" => Value::ForeignFunction(modules::require),
                "args" => Value::ForeignFunction(builtins::args),
                "call" => Value::ForeignFunction(builtins::call),
                "catch" => Value::ForeignFunction(builtins::catch),
                "def" => Value::ForeignFunction(builtins::def),
                "defglobal" => Value::ForeignFunction(builtins::defglobal),
                "list" => Value::ForeignFunction(builtins::list),
                "nil" => Value::ForeignFunction(builtins::nil),
                "throw" => Value::ForeignFunction(builtins::throw),
                "typeof" => Value::ForeignFunction(builtins::type_of),
                "modules" => Value::from(table! {
                    "loaded" => Value::from(table!()),
                    "loaders" => Value::Nil,
                }),
            },
        }
    }

    /// Register a module loader.
    pub fn module_loader(mut self, loader: ForeignFunction) -> Self {
        self.module_loaders.push(loader.into());
        self
    }

    pub fn with_stdlib(self) -> Self {
        self.module_loader(stdlib::stdlib_loader)
    }

    pub fn build(self) -> Runtime {
        self.globals
            .get("modules")
            .as_table()
            .unwrap()
            .set("loaders", Value::List(self.module_loaders));

        let mut runtime = Runtime {
            globals: Rc::new(self.globals),
            stack: Vec::new(),
            is_exiting: false,
            exit_code: 0,
        };

        runtime.init();

        runtime
    }
}

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    /// Table where global values are stored.
    globals: Rc<Table>,

    /// Current call stack.
    stack: Vec<Scope>,

    /// If application code inside the runtime requests the runtime to exit, this is set to true.
    is_exiting: bool,

    /// The runtime exit code to return after it shuts down.
    exit_code: i32,
}

impl Default for Runtime {
    fn default() -> Self {
        RuntimeBuilder::default().build()
    }
}

impl Runtime {
    /// Initialize the runtime environment.
    fn init(&mut self) {
        self.stack.push(Scope {
            args: Vec::new(),
            bindings: self.globals.clone(),
            parent: None,
        });
        self.globals.set("_GLOBALS", self.globals.clone());

        self.execute(None, include_str!("init.rip"))
            .expect("error in runtime initialization");
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

    /// Get the table that holds all global variables.
    pub fn globals(&self) -> &Table {
        &self.globals
    }

    /// Get a reference to the current scope.
    pub fn scope(&self) -> &Scope {
        self.stack.last().unwrap()
    }

    /// Execute the given script within this runtime.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub fn execute(&mut self, module: Option<&str>, file: impl Into<SourceFile>) -> Result<Value, Exception> {
        let block = match syntax::parse(file) {
            Ok(block) => block,
            Err(e) => throw!("error parsing: {}", e),
        };

        let closure = Closure {
            block: block,
            scope: None,
        };

        let module_scope = match module {
            Some(name) => {
                if self.globals.get("modules").get("loaded").get(name).is_nil() {
                    self.globals.get("modules").get("loaded").as_table().unwrap().set(name, table!());
                }

                self.globals.get("modules").get("loaded").get(name).as_table().unwrap().clone()
            },
            None => Rc::new(table!()),
        };

        // HACK...
        let parent = self.stack.last().cloned().map(Box::new);
        self.stack.push(Scope {
            args: Vec::new(),
            bindings: module_scope,
            parent: parent,
        });

        let result = self.invoke_closure(&closure, &[]);

        self.end_scope();

        result
    }

    /// Invoke the given value as a function with the given arguments.
    pub fn invoke(&mut self, value: &Value, args: &[Value]) -> Result<Value, Exception> {
        match value {
            Value::Block(closure) => {
                self.begin_scope(args.to_vec());
                let result = self.invoke_closure(closure, args);
                self.end_scope();
                result
            },
            Value::ForeignFunction(function) => (function)(self, args),
            _ => throw!("cannot invoke '{:?}' as a function", value),
        }
    }

    /// Invoke a block with an array of arguments.
    fn invoke_closure(&mut self, closure: &Closure, args: &[Value]) -> Result<Value, Exception> {
        let mut last_return_value = Value::Nil;

        // Evaluate each statement in order.
        for statement in closure.block.statements.iter() {
            match self.evaluate_pipeline(&statement) {
                Ok(return_value) => last_return_value = return_value,
                Err(exception) => {
                    // Exception thrown; abort and unwind stack.
                    self.end_scope();
                    return Err(exception);
                },
            }
        }

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
        let (function, args) = match call {
            Call::Named(path, args) => (self.scope().get_path(&path), args),
            Call::Unnamed(function, args) => (
                {
                    let mut function = self.evaluate_expr(*function)?;

                    // If the function is a string, resolve binding names first before we try to eval the item as a function.
                    if let Some(value) = function.as_string().map(|name| self.scope().get(name)) {
                        function = value;
                    }

                    function
                },
                args,
            ),
        };

        let mut arg_values = Vec::with_capacity(args.len());
        for expr in args {
            arg_values.push(self.evaluate_expr(expr)?);
        }

        self.invoke(&function, &arg_values)
    }

    fn evaluate_expr(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::Number(number) => Ok(Value::Number(number)),
            Expr::String(string) => Ok(Value::from(string)),
            // TODO: Handle expands
            Expr::InterpolatedString(_) => Ok(Value::Nil),
            Expr::Substitution(substitution) => self.evaluate_substitution(substitution),
            Expr::Block(block) => Ok(Value::from(Closure {
                block: block,
                scope: Some(self.scope().clone()),
            })),
            Expr::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline),
        }
    }

    fn evaluate_substitution(&mut self, substitution: Substitution) -> Result<Value, Exception> {
        match substitution {
            Substitution::Variable(path) => Ok(self.scope().get_path(&path)),
            Substitution::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline),
            _ => unimplemented!(),
        }
    }

    /// Open a new scope.
    fn begin_scope(&mut self, args: Vec<Value>) {
        let scope = Scope {
            args: args,
            bindings: Rc::new(table!()),
            parent: self.stack.last().cloned().map(Box::new),
        };
        self.stack.push(scope);
    }

    /// Close the current scope.
    fn end_scope(&mut self) {
        self.stack.pop();
    }
}

/// A function evaluation scope.
///
/// A scope encompasses the _environment_ in which functions are evaluated. Scopes are hierarchial, and contain a
/// reference to the enclosing, or parent, scope.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    args: Vec<Value>,
    bindings: Rc<Table>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    /// Get the arguments passed in to the current scope.
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        match self.bindings.get(name) {
            Value::Nil => match self.parent.as_ref() {
                Some(parent) => parent.get(name),
                None => Value::Nil,
            },
            value => value,
        }
    }

    pub fn get_path(&self, path: &VariablePath) -> Value {
        let mut result = self.get(&path.0[0]);

        if path.0.len() > 1 {
            for part in &path.0[1..] {
                result = result.get(part);
            }
        }

        result
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RString>, value: impl Into<Value>) {
        self.bindings.set(name, value);
    }
}
