//! The Riptide runtime.
use crate::builtins;
use crate::closure::Closure;
use crate::exceptions::Exception;
use crate::foreign::ForeignFn;
use crate::modules;
use crate::stdlib;
use crate::string::RipString;
use crate::syntax;
use crate::syntax::ast::*;
use crate::syntax::source::*;
use crate::table::Table;
use crate::value::*;
use futures::executor::block_on;
use futures::future::FutureExt;
use std::env;
use std::rc::Rc;
use std::time::Instant;

/// A function evaluation scope.
///
/// A scope encompasses the _environment_ in which functions are evaluated. Scopes are hierarchial, and contain a
/// reference to the enclosing, or parent, scope.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// The scope name, for debugging purposes.
    name: Option<String>,

    /// Arguments that were passed into this scope.
    args: Vec<Value>,

    /// Local scope bindings. May shadow bindings in the parent scope.
    pub(crate) bindings: Rc<Table>,

    /// A reference to the module this scope is executed in.
    pub(crate) module: Rc<Table>,

    /// The parent scope to this one.
    pub(crate) parent: Option<Rc<Scope>>,
}

impl Scope {
    /// Get the name of this scope, if available.
    pub fn name(&self) -> &str {
        self.name.as_ref().map(|s| &s as &str).unwrap_or("<unknown>")
    }

    /// Get the arguments passed in to the current scope.
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        if name == b"args" {
            return self.args.iter().cloned().collect();
        }

        if name == b"exports" {
            return self.module.clone().into();
        }

        match self.bindings.get(name) {
            Value::Nil => {},
            value => return value,
        };

        if let Some(parent) = self.parent.as_ref() {
            return parent.get(name);
        }

        Value::Nil
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        self.bindings.set(name, value);
    }
}

/// Configure a runtime.
pub struct RuntimeBuilder {
    module_loaders: Vec<ForeignFn>,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new().module_loader(modules::relative_loader).module_loader(modules::system_loader).with_stdlib()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            module_loaders: Vec::new(),
        }
    }

    /// Register a module loader.
    pub fn module_loader(mut self, loader: impl Into<ForeignFn>) -> Self {
        self.module_loaders.push(loader.into());
        self
    }

    pub fn with_stdlib(self) -> Self {
        self.module_loader(stdlib::stdlib_loader)
    }

    pub fn build(self) -> Runtime {
        let start_time = Instant::now();

        let mut runtime = Runtime {
            globals: Rc::new(Table::new()),
            stack: Vec::new(),
            exit_code: None,
        };

        // Set up globals
        runtime.globals.set("GLOBALS", runtime.globals.clone());
        runtime.globals.set("env", env::vars().collect::<Table>()); // Isn't that easy?

        // Initialize builtins
        builtins::init(&mut runtime);

        // Register predefined module loaders
        for loader in self.module_loaders {
            runtime.register_module_loader(loader);
        }

        // Execute initialization
        block_on(runtime.execute(None, include_str!("init.rip"))).expect("error in runtime initialization");

        log::debug!("runtime took {:?} to initialize", start_time.elapsed());

        runtime
    }
}

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    /// Table where global values are stored that are not on the stack.
    globals: Rc<Table>,

    /// Current call stack.
    ///
    /// This is exposed to the rest of the crate to support the `backtrace`
    /// function.
    pub(crate) stack: Vec<Rc<Scope>>,

    /// If the runtime has been requested to exit, this will be filled with the
    /// exit code to return after it shuts down.
    exit_code: Option<i32>,
}

impl Default for Runtime {
    fn default() -> Self {
        RuntimeBuilder::default().build()
    }
}

impl Runtime {
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    /// Register a module loader.
    pub fn register_module_loader(&self, loader: impl Into<ForeignFn>) {
        let modules = self.globals.get("modules").as_table().unwrap();
        modules.set("loaders", modules.get("loaders").append(loader.into()).unwrap());
    }

    /// Get the current exit code for the runtime. If no exit has been
    /// requested, then `None` will be returned.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Request the runtime to exit with the given exit code.
    ///
    /// The runtime will exit gracefully.
    pub fn exit(&mut self, code: i32) {
        log::debug!("runtime exit requested with exit code {}", code);

        match self.exit_code.take() {
            None => self.exit_code = Some(code),
            // Upgrade a zero exit code to a nonzero one.
            Some(0) => self.exit_code = Some(code),
            // Do not change an existing nonzero code if already exiting.
            Some(_) => {},
        }
    }

    /// Get the table that holds all global variables.
    pub fn globals(&self) -> &Table {
        &self.globals
    }

    /// Get the current executing scope.
    fn scope(&self) -> &Scope {
        self.stack.last().as_ref().unwrap()
    }

    /// Get the table that holds all global variables.
    pub(crate) fn module_scope(&self) -> &Table {
        &self.scope().module
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        if let Some(scope) = self.stack.last() {
            match scope.get(name) {
                Value::Nil => {},
                value => return value,
            }

            match scope.module.get(name) {
                Value::Nil => {},
                value => return value,
            }
        }

        self.globals.get(name)
    }

    fn get_path(&self, path: &VariablePath) -> Value {
        let mut result = self.get(&path.0[0]);

        if path.0.len() > 1 {
            for part in &path.0[1..] {
                result = result.get(part);
            }
        }

        result
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        self.scope().set(name, value);
    }

    pub(crate) fn set_parent(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        if self.stack.len() >= 2 {
            self.stack[self.stack.len() - 2].set(name, value);
        }
    }

    /// Compile the given source code as a closure.
    pub fn compile(&self, file: impl Into<SourceFile>, scope: Option<Rc<Table>>) -> Result<Closure, Exception> {
        let file = file.into();
        let file_name = file.name().to_string();

        let block = match syntax::parse(file) {
            Ok(block) => block,
            Err(e) => throw!("error parsing: {}", e),
        };

        let module_scope = self.get_module_by_name(&file_name);

        Ok(Closure {
            block: block,
            scope: Some(Rc::new(Scope {
                name: Some(format!("{}:<closure>", file_name)),
                args: Vec::new(),
                bindings: scope.unwrap_or_default(),
                module: module_scope,
                parent: None,
            })),
        })
    }

    /// Execute the given script within this runtime.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute(&mut self, module: Option<&str>, file: impl Into<SourceFile>) -> Result<Value, Exception> {
        self.execute_in_scope(module, file, Rc::new(table!())).await
    }

    /// Execute the given script using the given scope.
    ///
    /// The script will be executed inside the context of the module with the given name. If no module name is given, an
    /// anonymous module will be created for the file.
    ///
    /// If a compilation error occurs with the given file, an exception will be returned.
    pub async fn execute_in_scope(&mut self, module: Option<&str>, file: impl Into<SourceFile>, scope: Rc<Table>) -> Result<Value, Exception> {
        let closure = self.compile(file, Some(scope))?;

        self.invoke_closure(&closure, &[]).await
    }

    /// Invoke the given value as a function with the given arguments.
    pub async fn invoke(&mut self, value: &Value, args: &[Value]) -> Result<Value, Exception> {
        match value {
            Value::Block(closure) => self.invoke_closure(closure, args).boxed_local().await,
            Value::ForeignFn(function) => self.invoke_native(function, args).boxed_local().await,
            value => throw!("cannot invoke '{:?}' as a function", value),
        }
    }

    /// Get a module scope table by the module's name. If the module table does
    /// not already exist, it will be created.
    fn get_module_by_name(&self, name: &str) -> Rc<Table> {
        let loaded = self.globals.get("modules").get("loaded");

        if loaded.get(name).is_nil() {
            loaded.as_table().unwrap().set(name, table!());
        }

        loaded.get(name).as_table().unwrap().clone()
    }

    /// Invoke a block with an array of arguments.
    async fn invoke_closure(&mut self, closure: &Closure, args: &[Value]) -> Result<Value, Exception> {
        let mut scope = closure.scope.as_ref().unwrap().as_ref().clone();
        scope.args = args.to_vec();

        self.stack.push(Rc::new(scope));

        let mut last_return_value = Value::Nil;

        // Evaluate each statement in order.
        for statement in closure.block.statements.iter() {
            match self.evaluate_pipeline(&statement).await {
                Ok(return_value) => last_return_value = return_value,
                Err(exception) => {
                    // Exception thrown; abort and unwind stack.
                    self.stack.pop();
                    return Err(exception);
                }
            }
        }

        self.stack.pop();

        Ok(last_return_value)
    }

    /// Invoke a native function.
    async fn invoke_native(&mut self, function: &ForeignFn, args: &[Value]) -> Result<Value, Exception> {
        self.stack.push(Rc::new(Scope {
            name: Some(String::from("<native>")),
            args: args.to_vec(),
            bindings: Default::default(),
            module: Default::default(),
            parent: None,
        }));

        let result = function.call(self, &args).await;

        self.stack.pop();

        result
    }

    async fn evaluate_pipeline(&mut self, pipeline: &Pipeline) -> Result<Value, Exception> {
        // If there's only one call in the pipeline, we don't need to fork and can just execute the function by itself.
        if pipeline.0.len() == 1 {
            self.evaluate_call(pipeline.0[0].clone()).boxed_local().await
        } else {
            log::warn!("parallel pipelines not implemented!");
            Ok(Value::Nil)
        }
    }

    async fn evaluate_call(&mut self, call: Call) -> Result<Value, Exception> {
        let (function, args) = match call {
            Call::Named {function, args} => (self.get_path(&function), args),
            Call::Unnamed {function, args} => (
                {
                    let mut function = self.evaluate_expr(*function).await?;

                    // If the function is a string, resolve binding names first before we try to eval the item as a function.
                    if let Some(value) = function.as_string().map(|name| self.get(name)) {
                        function = value;
                    }

                    function
                },
                args,
            ),
        };

        let mut arg_values = Vec::with_capacity(args.len());
        for expr in args {
            arg_values.push(self.evaluate_expr(expr).await?);
        }

        self.invoke(&function, &arg_values).await
    }

    async fn evaluate_expr(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::Number(number) => Ok(Value::Number(number)),
            Expr::String(string) => Ok(Value::from(string)),
            Expr::Substitution(substitution) => self.evaluate_substitution(substitution).boxed_local().await,
            Expr::Table(literal) => self.evaluate_table_literal(literal).boxed_local().await,
            Expr::List(list) => self.evaluate_list_literal(list).boxed_local().await,
            // TODO: Handle expands
            Expr::InterpolatedString(_) => {
                log::warn!("string interpolation not yet supported");
                Ok(Value::Nil)
            },
            Expr::Block(block) => self.evaluate_block(block),
            Expr::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline).await,
        }
    }

    fn evaluate_block(&mut self, block: Block) -> Result<Value, Exception> {
        Ok(Value::from(Closure {
            block: block,
            scope: Some(Rc::new(Scope {
                name: Some(String::from("<closure>")),
                args: Vec::new(),
                bindings: Default::default(),
                module: self.scope().module.clone(),
                parent: self.stack.last().cloned(),
            })),
        }))
    }

    async fn evaluate_substitution(&mut self, substitution: Substitution) -> Result<Value, Exception> {
        match substitution {
            Substitution::Variable(path) => Ok(self.get_path(&path)),
            Substitution::Pipeline(ref pipeline) => self.evaluate_pipeline(pipeline).await,
            _ => unimplemented!(),
        }
    }

    async fn evaluate_table_literal(&mut self, literal: TableLiteral) -> Result<Value, Exception> {
        let table = Table::default();

        for entry in literal.0 {
            let key = self.evaluate_expr(entry.key).await?;
            let value = self.evaluate_expr(entry.value).await?;

            table.set(key.to_string(), value);
        }

        Ok(Value::from(table))
    }

    async fn evaluate_list_literal(&mut self, list: ListLiteral) -> Result<Value, Exception> {
        let mut values = Vec::new();

        for expr in list.0 {
            values.push(self.evaluate_expr(expr).await?);
        }

        Ok(Value::List(values))
    }
}
