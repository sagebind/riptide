//! A runtime table of global variable bindings.
//!
//! The global binding table is shared with all function calls and subshells (not sub-processes). Since crush uses many
//! threads to carry out subshell tasks, the global table is guarded with a mutex.
use expr::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};


lazy_static! {
    static ref TABLE: Mutex<HashMap<String, Arc<Expression>>> = Mutex::new(HashMap::new());
}

/// Get the value of a global binding.
pub fn get<S>(name: S) -> Option<Arc<Expression>>
    where S: AsRef<str>
{
    let table = TABLE.lock().unwrap();
    table.get(name.as_ref()).cloned()
}

/// Set the value of a global binding.
///
/// If a binding already exists for the given name, the old value is replaced and returned.
pub fn set<S>(name: S, value: Expression) -> Option<Arc<Expression>>
    where S: Into<String>
{
    let mut table = TABLE.lock().unwrap();
    table.insert(name.into(), Arc::new(value))
}
