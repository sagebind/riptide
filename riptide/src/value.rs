//! Structures and implementations of the built-in data types.
use ast;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

/// A Riptide value. This is a small enum that can rerepresent any of the possible data types. Since Riptide is loosely
/// typed, a value can be any of these types at runtime.
///
/// The "scalar" types are stored inline, while more heavyweight types are stored behind a pointer. This keeps the
/// memory footprint of a value small so it can be copied cheaply.
#[derive(Clone, Debug)]
pub enum Value {
    /// The "empty" value. This is equivalent to a unit type or "null" in some languages.
    Nil,

    /// A plain number. Stored by value.
    Number(Number),

    /// A string. Immutable, and stored by reference.
    String(RString),

    /// An immutable list of values. Stored by value.
    ///
    /// A vector is typically stored as a pointer and two integers. This seems to be a small enough size to store inline
    /// for now.
    List(Vec<Value>),

    /// A table, stored by reference.
    Table(Rc<Table>),

    /// A block, containing a list of expressions to execute. Stored by reference.
    Block(Rc<ast::Block>),
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Value::Number(value)
    }
}

impl From<&'static str> for Value {
    fn from(value: &'static str) -> Self {
        Value::String(RString::from(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(RString::from(value))
    }
}

impl From<ast::Block> for Value {
    fn from(block: ast::Block) -> Self {
        Value::Block(Rc::new(block))
    }
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

    /// If this value is a number, get its numeric value.
    pub fn as_number(&self) -> Option<Number> {
        match self {
            &Value::Number(number) => Some(number),
            _ => None,
        }
    }

    /// If this value is a string, get its string value.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            &Value::String(ref string) => Some(string),
            _ => None,
        }
    }

    /// If this value is a list, get its contents.
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            &Value::List(ref list) => Some(list),
            _ => None,
        }
    }

    /// If this value is a table, get a reference to it.
    pub fn as_table(&self) -> Option<Rc<Table>> {
        match self {
            &Value::Table(ref table) => Some(table.clone()),
            _ => None,
        }
    }

    /// Get a string representation of this value.
    pub fn to_string(&self) -> RString {
        match self {
            &Value::Nil => RString::EMPTY,
            &Value::Number(number) => number.to_string().into(),
            &Value::String(ref string) => string.clone(),
            &Value::List(_) => "<list>".into(),
            &Value::Table(_) => "<table>".into(),
            &Value::Block(_) => "<block>".into(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (&Value::Nil, &Value::Nil) => true,
            (&Value::Number(lhs), &Value::Number(rhs)) => lhs == rhs,
            (&Value::String(ref lhs), &Value::String(ref rhs)) => lhs == rhs,
            (&Value::List(ref lhs), &Value::List(ref rhs)) => lhs == rhs,
            (&Value::Table(ref lhs), &Value::Table(ref rhs)) => Rc::ptr_eq(lhs, rhs),
            (&Value::Block(ref lhs), &Value::Block(ref rhs)) => Rc::ptr_eq(lhs, rhs),
            _ => false,
        }
    }
}

impl<S> PartialEq<S> for Value where S: AsRef<str> {
    fn eq(&self, rhs: &S) -> bool {
        self.as_string() == Some(rhs.as_ref())
    }
}

/// A plain number.
pub type Number = f64;

/// A string value.
///
/// Since strings are copied and tossed around quite a bit, the string is
/// reference counted to reduce memory and copying.
#[derive(Clone, Debug, Eq)]
pub enum RString {
    Constant(&'static str),
    Heap(Rc<String>),
}

impl RString {
    /// The empty string.
    pub const EMPTY: Self = RString::Constant("");
}

impl From<&'static str> for RString {
    fn from(value: &'static str) -> Self {
        RString::Constant(value)
    }
}

impl From<String> for RString {
    fn from(value: String) -> Self {
        RString::Heap(Rc::new(value.into()))
    }
}

impl Deref for RString {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            &RString::Constant(s) => s,
            &RString::Heap(ref ptr) => ptr.as_ref(),
        }
    }
}

impl AsRef<str> for RString {
    fn as_ref(&self) -> &str {
        &*self
    }
}

impl Borrow<str> for RString {
    fn borrow(&self) -> &str {
        &*self
    }
}

impl PartialEq for RString {
    fn eq(&self, rhs: &RString) -> bool {
        let lhs = self.as_ref();
        let rhs = rhs.as_ref();

        // First compare by address.
        if lhs as *const _ == rhs as *const _ {
            return true;
        }

        // Compare by string contents.
        lhs == rhs
    }
}

impl PartialEq<str> for RString {
    fn eq(&self, rhs: &str) -> bool {
        self.as_ref() == rhs
    }
}

impl Hash for RString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone, Debug)]
pub struct Table {
    /// Internally a hashmap is used, but the implementation could vary.
    ///
    /// Unlike all other value types, tables are mutable, so we are using a cell here to implement that.
    map: RefCell<HashMap<RString, Value>>,
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Table {
            map: RefCell::new(HashMap::new()),
        }
    }

    /// Get the value indexed by a key.
    ///
    /// If the key does not exist, `Nil` is returned.
    pub fn get(&self, key: &str) -> Value {
        self.map.borrow().get(key).cloned().unwrap_or(Value::Nil)
    }

    /// Set the value for a given key.
    ///
    /// If `Nil` is given as the value, the key is unset.
    pub fn set<V: Into<Value>>(&mut self, key: &str, value: V) -> Option<Value> {
        let value = value.into();

        match value {
            Value::Nil => self.map.borrow_mut().remove(key),
            value => self.map.borrow_mut().insert(RString::from(String::from(key)), value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        let mut table = Table::new();

        assert!(table.get("foo") == Value::Nil);
        assert!(table.set("foo", "hello").is_none());
        assert!(table.get("foo") == "hello");
        assert!(table.get("foo") == Value::from("hello"));
    }
}
