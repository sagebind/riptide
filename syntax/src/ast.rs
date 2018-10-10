//! Abstract syntax tree definitions for the language syntax.

use std::fmt;

/// A function block, containing a list of pipelines to execute.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// A list of named parameters.
    pub named_params: Option<Vec<String>>,

    /// A list of statements to execute.
    pub statements: Vec<Pipeline>,
}

/// A pipeline of function calls.
#[derive(Clone, PartialEq)]
pub struct Pipeline {
    pub items: Vec<Call>,
}

impl fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Pipeline ").and_then(|_|
            f.debug_list().entries(&self.items).finish())
    }
}

/// A function call.
#[derive(Clone, PartialEq)]
pub enum Call {
    /// A function call for a named function.
    Named(VariablePath, Vec<Expr>),

    /// A function call on a callable object.
    Unnamed(Box<Expr>, Vec<Expr>),
}

impl fmt::Debug for Call {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Call::Named(path, args) => f.debug_struct("Call::Named")
                .field("function", path)
                .field("args", args)
                .finish(),
            Call::Unnamed(func, args) => f.debug_struct("Call::Unnamed")
                .field("function", func)
                .field("args", args)
                .finish(),
        }
    }
}

/// Abstract representation of an expression.
///
/// Contains a variant for each different expression type.
#[derive(Clone, PartialEq)]
pub enum Expr {
    /// A number literal.
    Number(f64),

    /// A string literal.
    String(String),

    /// A substitution.
    Substitution(Substitution),

    /// An interpolated string literal.
    InterpolatedString(InterpolatedString),

    /// A function block.
    Block(Block),

    /// A function pipeline.
    Pipeline(Pipeline),
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Number(v) => write!(f, "Number({})", v),
            Expr::String(v) => write!(f, "String({:?})", v),
            Expr::Substitution(v) => f.debug_tuple("Substitution").field(v).finish(),
            Expr::InterpolatedString(v) => v.fmt(f),
            Expr::Block(v) => v.fmt(f),
            Expr::Pipeline(v) => v.fmt(f),
        }
    }
}

/// Value substitution.
#[derive(Clone, Debug, PartialEq)]
pub enum Substitution {
    /// A format substitution with a variable and parameters, such as `${foo:.2}`.
    ///
    /// This always evaluates to a string, unless the referenced variable is not defined.
    Format(VariablePath, Option<String>),

    /// A pipeline substitution, such as `$(add 1 2 3)`.
    ///
    /// Evaluates to the final return value after executing the pipeline.
    Pipeline(Pipeline),

    /// A simple variable substitution, such as `$foo`.
    ///
    /// This gets evaluated to the current value of the variable identified.
    Variable(VariablePath),
}

#[derive(Clone, Eq, PartialEq)]
pub struct VariablePath(pub Vec<String>);

impl fmt::Debug for VariablePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VariablePath ").and_then(|_|
            f.debug_list().entries(&self.0).finish())
    }
}

impl fmt::Display for VariablePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.join("->"))
    }
}

/// An interpolated string literal.
///
/// An interpolated string is made up of a sequence of parts that, when stringified and concatenated in order, form the
/// desired string value.
#[derive(Clone, PartialEq)]
pub struct InterpolatedString(pub Vec<InterpolatedStringPart>);

impl fmt::Debug for InterpolatedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InterpolatedString ").and_then(|_|
            f.debug_list().entries(&self.0).finish())
    }
}

#[derive(Clone, PartialEq)]
pub enum InterpolatedStringPart {
    String(String),
    Substitution(Substitution),
}

impl fmt::Debug for InterpolatedStringPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InterpolatedStringPart::String(v) => write!(f, "String({:?})", v),
            InterpolatedStringPart::Substitution(v) => f.debug_tuple("Substitution").field(v).finish(),
        }
    }
}
