use super::{string::RipString, table::Table, value::Value};
use std::rc::Rc;

/// A function evaluation scope.
///
/// A scope encompasses the _environment_ in which functions are evaluated. Scopes are hierarchial, and contain a
/// reference to the enclosing, or parent, scope.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    /// The scope name, for debugging purposes.
    pub(crate) name: Option<String>,

    /// Local scope bindings. May shadow bindings in the parent scope.
    pub(crate) bindings: Table,

    /// A reference to the module this scope is executed in.
    pub(crate) module: Table,

    /// The parent scope to this one.
    pub(crate) parent: Option<Rc<Scope>>,
}

impl Scope {
    /// Get the name of this scope, if available.
    pub fn name(&self) -> &str {
        self.name
            .as_ref()
            .map(|s| &s as &str)
            .unwrap_or("<unknown>")
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        if name == b"exports" {
            return self.module.clone().into();
        }

        match self.bindings.get(name) {
            Value::Nil => {}
            value => return value,
        };

        if let Some(parent) = self.parent.as_ref() {
            return parent.get(name);
        }

        match self.module.get(name) {
            Value::Nil => {}
            value => return value,
        };

        Value::Nil
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        self.bindings.set(name, value);
    }
}
