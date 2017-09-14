//! Definition and structures for the runtime respresentation (RR) of a program.
//!
//! Interestingly, the RR acts as an abstract syntax tree (AST) as well, since
//! the language is entirely expression-based.
use std::sync::Arc;


/// Abstract representation of an expression.
///
/// Contains a variant for each different expression type.
#[derive(Clone, Debug)]
pub enum Expr {
    /// The "empty" value. This is equivalent to a unit type or "null" in some languages.
    Nil,

    /// A string value.
    String(RString),

    /// A list of values.
    List(RList),

    /// A function call.
    Call {
        /// The function to invoke. Could be a binding name or a block.
        function: Expr,

        /// A list of arguments to pass to the function.
        args: RList,
    },

    /// A function block, containing a list of expressions to execute.
    Block(RList),
}

impl<S> From<S> for Expr where S: Into<RString> {
    fn from(value: S) -> Self {
        Expr::String(value.into())
    }
}

impl Expr {
    /// Determine if this expression is considered a truthy value.
    ///
    /// Nil, the empty string, and the empty list are considered falsey, and all
    /// other values are considered truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Expr::Nil => false,
            &Expr::Atom(ref value) => !(value == "0" || value.is_empty() || value.to_lowercase() == "false"),
            &Expr::List(ref items) => !items.is_empty(),
            _ => true,
        }
    }

    pub fn as_string(&self) -> Option<&RString> {
        match self {
            &Expr::String(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&RList> {
        match self {
            &Expr::List(ref l) => Some(l),
            _ => None,
        }
    }
}


/// A string value.
///
/// Since strings are copied and tossed around quite a bit, the string is
/// reference counted to reduce memory and copying.
#[derive(Clone, Debug)]
pub struct RString(Arc<String>);

impl<S> From<S> for RString where S: Into<String> {
    fn from(value: S) -> Self {
        RString(Arc::new(value.into()))
    }
}

impl AsRef<str> for RString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}


/// A list of values.
#[derive(Clone, Debug)]
pub struct RList(Vec<Expr>);
