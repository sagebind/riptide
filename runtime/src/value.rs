//! Structures and implementations of the built-in data types.
use super::{
    closure::Closure,
    foreign::ForeignFn,
    string::RipString,
    table::Table,
};
use std::{
    fmt,
    iter::FromIterator,
};

type Number = f64;

/// A Riptide value. This is a small enum that can represent any of the possible data types. Since Riptide is loosely
/// typed, a value can be any of these types at runtime.
///
/// The "scalar" types are stored inline, while more heavyweight types are stored behind a pointer. This keeps the
/// memory footprint of a value small so it can be copied cheaply.
#[derive(Clone, gc::Finalize, gc::Trace)]
pub enum Value {
    /// The "empty" value. This is equivalent to a unit type or "null" in some languages.
    Nil,

    /// A boolean value.
    Boolean(bool),

    /// A plain number. Stored by value.
    Number(Number),

    /// A string. Immutable, and stored by reference.
    String(RipString),

    /// An immutable list of values. Stored by value.
    ///
    /// A vector is typically stored as a pointer and two integers. This seems to be a small enough size to store inline
    /// for now.
    List(Vec<Value>),

    /// A table, stored by reference.
    Table(Table),

    /// A block, containing a list of expressions to execute. Stored by reference.
    Block(Closure),

    /// Reference to a foreign (native) function.
    ForeignFn(#[unsafe_ignore_trace] ForeignFn),
}

impl Value {
    pub const TRUE: Self = Value::Boolean(true);
    pub const FALSE: Self = Value::Boolean(false);

    pub fn foreign_fn(function: impl Into<ForeignFn>) -> Self {
        Value::ForeignFn(function.into())
    }

    /// Get the type of value, rendered as a string.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Boolean(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Table(_) => "table",
            Value::Block(_) => "block",
            Value::ForeignFn(_) => "native",
        }
    }

    pub fn is_nil(&self) -> bool {
        match self {
            Value::Nil => true,
            _ => false,
        }
    }

    /// Determine if this expression is considered a truthy value.
    ///
    /// Nil, the empty string, and the empty list are considered falsey, and all
    /// other values are considered truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Boolean(b) => *b,
            Value::String(value) => !(value == "0" || value.as_bytes().is_empty() || &value.to_lowercase() == "false"),
            Value::List(items) => !items.is_empty(),
            _ => true,
        }
    }

    /// If this value is a boolean, get its value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// If this value is a number, get its numeric value.
    pub fn as_number(&self) -> Option<Number> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// If this value is a string, get its string value.
    pub fn as_string(&self) -> Option<&RipString> {
        match self {
            Value::String(string) => Some(string),
            _ => None,
        }
    }

    /// If this value is a list, get its contents.
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }

    /// If this value is a table, get a reference to it.
    pub fn as_table(&self) -> Option<Table> {
        match self {
            Value::Table(table) => Some(table.clone()),
            _ => None,
        }
    }

    /// If this is a table, get the value indexed by a key.
    pub fn get(&self, key: impl AsRef<[u8]>) -> Value {
        self.as_table().map(|t| t.get(key)).unwrap_or(Value::Nil)
    }

    /// If this is a list, return a new list with the given value appended.
    pub fn append(&self, value: impl Into<Value>) -> Option<Value> {
        match self {
            Value::List(items) => {
                let mut new = items.clone();
                new.push(value.into());
                Some(Value::List(new))
            },
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Value::Number(value)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Number(value as Number)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Number(value as Number)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &str) -> Self {
        Value::String(RipString::from(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(RipString::from(value))
    }
}

impl<'a> From<&'a bstr::BStr> for Value {
    fn from(value: &bstr::BStr) -> Self {
        Value::String(RipString::from(value))
    }
}

impl From<bstr::BString> for Value {
    fn from(value: bstr::BString) -> Self {
        Value::String(RipString::from(value))
    }
}

impl From<RipString> for Value {
    fn from(value: RipString) -> Self {
        Value::String(value)
    }
}

impl From<std::path::PathBuf> for Value {
    fn from(value: std::path::PathBuf) -> Self {
        Value::String(RipString::from(value.as_os_str()))
    }
}

impl From<Vec<Value>> for Value {
    fn from(list: Vec<Value>) -> Self {
        Value::List(list)
    }
}

impl<'a> From<&'a [Value]> for Value {
    fn from(list: &[Value]) -> Self {
        Value::List(list.to_vec())
    }
}

impl From<Table> for Value {
    fn from(table: Table) -> Self {
        Value::Table(table)
    }
}

impl From<Closure> for Value {
    fn from(closure: Closure) -> Self {
        Value::Block(closure)
    }
}

impl From<ForeignFn> for Value {
    fn from(f: ForeignFn) -> Self {
        Value::ForeignFn(f)
    }
}

impl From<Value> for RipString {
    fn from(value: Value) -> Self {
        match &value {
            Value::String(s) => s.clone(),
            value => value.to_string().into(),
        }
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        Value::List(iter.into_iter().map(Into::into).collect())
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(lhs), Value::Boolean(rhs)) => lhs == rhs,
            (Value::Number(lhs), Value::Number(rhs)) => lhs == rhs,
            (Value::String(lhs), Value::String(rhs)) => lhs == rhs,
            (Value::List(lhs), Value::List(rhs)) => lhs == rhs,
            (Value::Table(lhs), Value::Table(rhs)) => lhs == rhs,
            (Value::Block(lhs), Value::Block(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl PartialEq<Number> for Value {
    fn eq(&self, &rhs: &Number) -> bool {
        self.as_number() == Some(rhs)
    }
}

impl PartialEq<Number> for &Value {
    fn eq(&self, &rhs: &Number) -> bool {
        **self == rhs
    }
}

impl PartialEq<[u8]> for Value {
    fn eq(&self, rhs: &[u8]) -> bool {
        match self.as_string() {
            Some(s) => s == rhs,
            None => false,
        }
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, rhs: &str) -> bool {
        self == rhs.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Value {
    fn eq(&self, rhs: &&str) -> bool {
        self == rhs.as_bytes()
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, rhs: &String) -> bool {
        self == rhs.as_bytes()
    }
}

impl PartialEq<RipString> for Value {
    fn eq(&self, rhs: &RipString) -> bool {
        match self.as_string() {
            Some(s) => s == rhs,
            None => false,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::Number(number) => write!(f, "{}", number),
            Value::String(string) => write!(f, "\"{}\"", string),
            _ => write!(f, "<{}>", self.type_name()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::Number(number) => write!(f, "{}", number),
            Value::String(string) => write!(f, "{}", string),
            Value::List(items) => {
                write!(f, "[")?;
                let mut first = true;

                for item in items {
                    if first {
                        write!(f, "{}", item)?;
                        first = false;
                    } else {
                        write!(f, ",{}", item)?;
                    }
                }

                write!(f, "]")
            }
            Value::Table(table) => write!(f, "{}", table),
            _ => write!(f, "<{}>", self.type_name()),
        }
    }
}
