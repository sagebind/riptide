//! Abstract syntax tree.
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
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// The function to invoke. Could be a binding name or a block.
    pub function: Box<Expr>,

    /// A list of argument expressions to pass to the function.
    pub args: Vec<Expr>,
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
    /// A simple variable substitution, such as `$foo`.
    ///
    /// This gets evaluated to the current value of the variable identified.
    Variable(VariablePath),

    /// A format substitution with a variable and parameters, such as `${foo:.2}`.
    ///
    /// This always evaluates to a string, unless the referenced variable is not defined.
    Format(VariablePath, Option<String>),

    /// A pipeline substitution, such as `$(add 1 2 3)`.
    ///
    /// Evaluates to the final return value after executing the pipeline.
    Pipeline(Pipeline),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariablePath(pub Vec<VariablePathPart>);

impl fmt::Display for VariablePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<String>>()
            .join("."))
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum VariablePathPart {
    /// An identifier referencing a variable by name.
    Ident(String),
}

impl fmt::Debug for VariablePathPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ident({})", match self {
            VariablePathPart::Ident(s) => s,
        })
    }
}

impl fmt::Display for VariablePathPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            VariablePathPart::Ident(s) => s,
        })
    }
}

/// An interpolated string literal.
///
/// An interpolated string is made up of a sequence of parts that, when stringified and concatenated in order, form the
/// desired string value.
#[derive(Clone, Debug, PartialEq)]
pub struct InterpolatedString(Vec<InterpolatedStringPart>);

#[derive(Clone, Debug, PartialEq)]
pub enum InterpolatedStringPart {
    String(String),
    Substitution(Substitution),
}
