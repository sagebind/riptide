use interpreter;
use interpreter::Expression;
use io::IO;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};


/// Global table of user-defined functions.
lazy_static! {
    static ref TABLE: Mutex<HashMap<String, Arc<UserFunction>>> = Mutex::new(HashMap::new());
}


/// A shell function.
pub trait Function {
    /// Execute the function in the given IO context and capture its return value.
    fn execute(&self, args: &[Expression], io: &mut IO) -> Expression;
}


/// A builtin function in native code.
///
/// Builtin functions have the special property of receiving their arguments before they are reduced.
pub type Builtin = fn(&[Expression], &mut IO) -> Expression;

impl Function for Builtin {
    fn execute(&self, args: &[Expression], io: &mut IO) -> Expression {
        self(args, io)
    }
}


/// A function defined by the user.
///
/// User functions consist of an argument list, and a body. The body is an expression, which gets executed on every
/// invocation. The argument list is a list of names that are used inside the body. These names actually become function
/// aliases for the expressions passed in at call time. Thus, if an argument is never used, it is never executed.
pub struct UserFunction {
    args: Vec<String>,
    pub body: Expression,
}

impl Function for UserFunction {
    fn execute(&self, args: &[Expression], io: &mut IO) -> Expression {
        interpreter::execute(&self.body, io)
    }
}


/// Lookup a user-defined function by name.
pub fn lookup<S>(name: S) -> Option<Arc<UserFunction>>
    where S: AsRef<str>
{
    let table = TABLE.lock().unwrap();
    table.get(name.as_ref()).cloned()
}

/// Create a new user-defined function.
pub fn create<S>(name: S, body: Expression)
    where S: Into<String>
{
    let function = UserFunction {
        args: Vec::new(),
        body: body,
    };

    let mut table = TABLE.lock().unwrap();
    table.insert(name.into(), Arc::new(function));
}
