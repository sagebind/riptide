use execute;
use io::IO;
use parser::Expression;
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
pub struct UserFunction {
    body: Expression,
}

impl Function for UserFunction {
    fn execute(&self, args: &[Expression], io: &mut IO) -> Expression {
        execute::reduce(&self.body, io)
    }
}


/// Lookup a user-defined function by name.
pub fn lookup<S>(name: S) -> Option<Arc<UserFunction>>
    where S: AsRef<str>
{
    let mut table = TABLE.lock().unwrap();
    table.get(name.as_ref()).cloned()
}

/// Create a new user-defined function.
pub fn create<S>(name: S, body: Expression)
    where S: Into<String>
{
    let function = UserFunction {
        body: body,
    };

    let mut table = TABLE.lock().unwrap();
    table.insert(name.into(), Arc::new(function));
}
