//! Structures and implementations of the built-in data types.
use riptide_syntax::ast;
use runtime::ForeignFunction;
use std::fmt;
use std::rc::Rc;
use self::string::RString;
use self::table::Table;

pub mod string;
pub mod table;

/// A plain number.
pub type Number = f64;

/// A Riptide value. This is a small enum that can rerepresent any of the possible data types. Since Riptide is loosely
/// typed, a value can be any of these types at runtime.
///
/// The "scalar" types are stored inline, while more heavyweight types are stored behind a pointer. This keeps the
/// memory footprint of a value small so it can be copied cheaply.
#[derive(Clone)]
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

    /// Reference to a foreign (native) function.
    ForeignFunction(ForeignFunction),
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

impl From<ForeignFunction> for Value {
    fn from(function: ForeignFunction) -> Self {
        Value::ForeignFunction(function)
    }
}

impl Value {
    /// Get the type of value, rendered as a string.
    pub fn type_name(&self) -> &'static str {
        match self {
            &Value::Nil => "nil",
            &Value::Number(_) => "number",
            &Value::String(_) => "string",
            &Value::List(_) => "list",
            &Value::Table(_) => "table",
            &Value::Block(_) => "block",
            &Value::ForeignFunction(_) => "native",
        }
    }

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
            _ => RString::from(format!("{:?}", self)),
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

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "nil"),
            &Value::Number(number) => write!(f, "{}", number),
            &Value::String(ref string) => write!(f, "\"{}\"", string),
            _ => write!(f, "<{}>", self.type_name()),
        }
    }
}
