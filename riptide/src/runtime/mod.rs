//! The Riptide runtime.
use ast::*;
use self::value::Value;
use self::value::table::Table;

pub mod value;

pub type ForeignFunction = fn(&mut Runtime, &[Value]) -> Result<Value, Exception>;

#[derive(Clone, Debug)]
pub struct Exception(Value);

/// Holds all of the state of a Riptide runtime.
pub struct Runtime {
    /// Holds global variable bindings.
    globals: Table,

    /// Function call stack containing call frames.
    call_stack: Vec<CallFrame>,
}

/// Contains information about the current function call.
struct CallFrame {
    args: Vec<Value>,
    bindings: Table,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            globals: Table::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn get_global(&self, name: &str) -> Value {
        self.globals.get(name)
    }

    pub fn set_global<V: Into<Value>>(&mut self, name: &str, value: V) {
        self.globals.set(name, value);
    }

    /// Evaluate the given expression, returning the result.
    ///
    /// This function is re-entrant.
    pub fn evaluate(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::String(string) => Ok(Value::from(string)),

            // TODO: Handle expands
            Expr::ExpandableString(string) => Ok(Value::from(string)),

            Expr::Block(block) => Ok(Value::from(block)),

            Expr::Call(call) => {
                let mut function = self.evaluate(*call.function)?;

                let mut args = Vec::with_capacity(call.args.len());
                for expr in call.args {
                    args.push(self.evaluate(expr)?);
                }

                // If the function is a string, resolve binding names first before we try to eval the item as a function.
                if let Some(mut value) = function.as_string().and_then(|name| self.resolve(name)) {
                    function = value;
                }

                // Execute the function.
                match function {
                    Value::Block(block) => {
                        self.call_stack.push(CallFrame {
                            args: args,
                            bindings: Table::new(),
                        });

                        let mut r = Value::Nil;

                        for statement in block.statements.iter().rev() {
                            r = self.evaluate(statement.clone())?;
                        }

                        self.call_stack.pop();

                        Ok(r)
                    },
                    Value::ForeignFunction(f) => {
                        f(self, &args)
                    },
                    _ => Err(Exception(Value::from(format!("cannot execute {:?} as a function", function)))),
                }
            },
        }
    }

    fn resolve(&self, name: &str) -> Option<Value> {
        for frame in self.call_stack.iter().rev() {
            let value = frame.bindings.get(name);

            if value != Value::Nil {
                return Some(value);
            }
        }

        let value = self.get_global(name);
        if value != Value::Nil {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use ast::Expr;
    use super::*;

    #[test]
    fn basic() {
        let mut runtime = Runtime::new();

        runtime.evaluate(Expr::Call(Call {
            function: Box::new(Expr::String("println".into())),
            args: vec![
                Expr::String("hello".into()),
                Expr::Call(Call {
                    function: Box::new(Expr::Block(Block {
                        statements: vec![
                            Expr::String("read".into()),
                        ],
                    })),
                    args: vec![
                        Expr::String("THE".into()),
                    ],
                }),
                Expr::Call(Call {
                    function: Box::new(Expr::String("uppercase".into())),
                    args: vec![
                        Expr::String("World".into()),
                    ],
                }),
            ],
        })).unwrap();
    }
}
