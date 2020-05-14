use super::{string::RipString, table::Table, value::Value};
use std::rc::Rc;

/// A function evaluation scope.
///
/// A scope encompasses the _environment_ in which functions are evaluated.
/// Scopes are hierarchial, and contain a reference to the enclosing, or parent,
/// scope.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    /// The scope name, for debugging purposes.
    pub(crate) name: String,

    /// Local scope bindings. May shadow bindings in the parent scope.
    pub(crate) bindings: Table,

    /// Context variables have an entirely separate namespace from normal
    /// variables, and they are stored here.
    ///
    /// This table never changes during the lifetime of the scope; cvars are set
    /// on creation of the scope.
    pub(crate) cvars: Table,

    /// The lexically parent scope to this one.
    pub(crate) parent: Option<Rc<Scope>>,
}

impl Scope {
    /// Get the name of this scope, if available.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Lookup a variable name in the current scope.
    pub fn get(&self, name: impl AsRef<[u8]>) -> Value {
        let name = name.as_ref();

        match self.bindings.get(name) {
            Value::Nil => {}
            value => return value,
        };

        if let Some(parent) = self.parent.as_ref() {
            return parent.get(name);
        }

        Value::Nil
    }

    /// Set a variable value in the current scope.
    pub fn set(&self, name: impl Into<RipString>, value: impl Into<Value>) {
        // TODO: Handle concept of assigning to existing variables and not just
        // declaring new ones.
        self.bindings.set(name, value);
    }
}
