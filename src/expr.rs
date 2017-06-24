//! In-memory representation of expression trees.
//!
//! Everything is represented as expressions, both source code and data. An expression tree is used as an AST when a
//! script is parsed. The same expression tree becomes the memory model for a running script later, which functions
//! operate on.
use std::borrow::Cow;
use std::cmp;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;


#[derive(Clone, Debug)]
/// Abstract representation of an expression.
pub enum Expression {
    /// An empty value. This is equivalent to a unit type or "null" in some languages.
    Nil,

    /// A single value. Values are always represented as strings.
    Atom(Cow<'static, str>),

    /// An ordered list of expressions.
    List(Vec<Expression>),

    /// A lambda function.
    ///
    /// Lambdas are not put into an expression tree at parse time. Instead, they are created at runtime using the
    /// `lambda` builtin function instead.
    Lambda(Arc<Function>),
}

/// Structure of a function definition.
///
/// User functions consist of an argument list, and a body. The body is an expression, which gets executed on every
/// invocation. The argument list is a list of names that are used inside the body. These names actually become function
/// aliases for the expressions passed in at call time.
#[derive(Clone, Debug)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Expression,
}



impl Expression {
    pub const TRUE: Self = Expression::Atom(Cow::Borrowed("true"));
    pub const FALSE: Self = Expression::Atom(Cow::Borrowed("false"));

    /// Create a new atom.
    pub fn atom<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Expression::Atom(value.into())
    }

    /// Create a new lambda function.
    pub fn create_lambda(params: Vec<String>, body: Expression) -> Self {
        Expression::Lambda(Arc::new(Function {
            params: params,
            body: body,
        }))
    }

    /// Determine if this expression is equivalent to Nil, or ().
    pub fn is_nil(&self) -> bool {
        match self {
            &Expression::Nil => true,
            &Expression::List(ref items) => items.is_empty(),
            _ => false,
        }
    }

    /// Determine if this expression is considered a truthy value.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Expression::Nil => false,
            &Expression::Atom(ref value) => !(value == "0" || value.is_empty() || value.to_lowercase() == "false"),
            &Expression::List(ref items) => !items.is_empty(),
            &Expression::Lambda {..} => true,
        }
    }

    /// Try to parse this expression as a value into another type.
    pub fn parse<F: FromStr>(&self) -> Option<F> {
        match self.as_value() {
            Some(s) => s.parse().ok(),
            None => None,
        }
    }

    /// If this is an atom expression, get its value.
    pub fn as_value(&self) -> Option<&str> {
        if let &Expression::Atom(ref s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// If this is a non-empty list, return a reference to its contents.
    pub fn as_items(&self) -> Option<&[Expression]> {
        match self {
            &Expression::List(ref items) if items.len() > 0 => Some(items),
            _ => None,
        }
    }

    /// If this is a lambda, get its function definition.
    pub fn as_lambda(&self) -> Option<Arc<Function>> {
        match self {
            &Expression::Lambda(ref function) => Some(function.clone()),
            _ => None,
        }
    }

    /// Attempt to decompose the expression into its items if it is a list.
    pub fn into_items(self) -> Result<Vec<Expression>, Self> {
        if self.as_items().is_some() {
            if let Expression::List(items) = self {
                return Ok(items);
            }
        }

        Err(self)
    }
}

impl cmp::PartialEq for Expression {
    fn eq(&self, rhs: &Self) -> bool {
        (self.is_nil() && rhs.is_nil()) || (self.as_value() == rhs.as_value())
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Expression::Nil => write!(f, "()"),
            &Expression::Atom(ref s) => write!(f, "{}", s),
            &Expression::List(ref v) => {
                write!(f, "(")?;
                let mut first = true;
                for expr in v {
                    if first {
                        write!(f, "{}", expr)?;
                        first = false;
                    } else {
                        write!(f, " {}", expr)?;
                    }
                }
                write!(f, ")")
            },
            &Expression::Lambda {..} => write!(f, "#lambda#"),
        }
    }
}

impl<'e> Into<Cow<'e, Expression>> for Expression {
    fn into(self) -> Cow<'e, Expression> {
        Cow::Owned(self)
    }
}

impl<'e> Into<Cow<'e, Expression>> for &'e Expression {
    fn into(self) -> Cow<'e, Expression> {
        Cow::Borrowed(self)
    }
}
