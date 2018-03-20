//! Definition and structures for the runtime respresentation (RR) of a program.
//!
//! Interestingly, the RR is used in the abstract syntax tree (AST) as well, since
//! the language is entirely expression-based.
use ast::Expr;
use std::collections::HashMap;
use std::sync::Arc;

/// A runtime value.
#[derive(Clone, Debug)]
pub enum Value {
    /// The "empty" value. This is equivalent to a unit type or "null" in some languages.
    Nil,

    /// A string.
    String(RString),

    /// A list of values.
    List(Vec<Value>),

    /// A table, stored as a hash map.
    Table(HashMap<String, Value>),

    /// A block, containing a list of expressions to execute.
    Block(Vec<Expr>),
}

impl Value {
    /// Determine if this expression is considered a truthy value.
    ///
    /// Nil, the empty string, and the empty list are considered falsey, and all
    /// other values are considered truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Value::Nil => false,
            &Value::String(ref value) => {
                !(value.as_ref() == "0" || value.is_empty() || value.to_lowercase() == "false")
            }
            &Value::List(ref items) => !items.is_empty(),
            _ => true,
        }
    }

    pub fn as_string(&self) -> Option<&RString> {
        match self {
            &Value::String(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&RList> {
        match self {
            &Value::List(ref l) => Some(l),
            _ => None,
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value.into())
    }
}

/// A string value.
///
/// Since strings are copied and tossed around quite a bit, the string is
/// reference counted to reduce memory and copying.
#[derive(Clone, Debug)]
pub struct RString(Arc<String>);

impl<S> From<S> for RString
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        RString(Arc::new(value.into()))
    }
}

impl AsRef<str> for RString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
