use ast::*;
use value::Value;
use value::table::Table;

pub type ForeignFunction = fn(&mut Interpreter, &[Value]) -> Result<Value, Exception>;

pub struct Interpreter {
    globals: Table,
    stack: Vec<CallFrame>,
}

pub struct CallFrame {
    pub args: Vec<Value>,
    bindings: Table,
}

#[derive(Clone, Debug)]
pub struct Exception(Value);

impl<V: Into<Value>> From<V> for Exception {
    fn from(message: V) -> Self {
        Exception(message.into())
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            globals: Table::new(),
            stack: Vec::new(),
        }
    }

    pub fn frame(&self) -> &CallFrame {
        self.stack.first().unwrap()
    }

    pub fn get_global(&self, name: &str) -> Value {
        self.globals.get(name)
    }

    pub fn set_global<V: Into<Value>>(&mut self, name: &str, value: V) {
        self.globals.set(name, value);
    }

    pub fn execute(&mut self, expr: Expr) -> Result<Value, Exception> {
        match expr {
            Expr::String(string) => Ok(Value::from(string)),

            // TODO: Handle expands
            Expr::ExpandableString(string) => Ok(Value::from(string)),

            Expr::Block(block) => Ok(Value::from(block)),

            Expr::Call(call) => {
                let mut function = self.execute(*call.function)?;

                let mut args = Vec::with_capacity(call.args.len());
                for expr in call.args {
                    args.push(self.execute(expr)?);
                }

                // If the function is a string, resolve binding names first before we try to execute the item as a function.
                if let Some(mut value) = function.as_string().and_then(|name| self.binding_lookup(name)) {
                    function = value;
                }

                self.stack.push(CallFrame {
                    args: args,
                    bindings: Table::new(),
                });

                // Execute the function.
                let return_value = match function {
                    Value::Block(block) => {
                        let mut r = Value::Nil;

                        for statement in block.statements.iter().rev() {
                            r = self.execute(statement.clone())?;
                        }

                        r
                    },
                    Value::ForeignFunction(f) => f(self, &self.stack.last().unwrap().args)?,
                    _ => {
                        return Err(Exception::from(format!("cannot execute {:?} as a function", function)));
                    },
                };

                self.stack.pop();

                Ok(return_value)
            },
        }
    }

    fn binding_lookup(&self, name: &str) -> Option<Value> {
        for frame in self.stack.iter().rev() {
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
        let mut interpreter = Interpreter::new();

        interpreter.execute(Expr::Call(Call {
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
