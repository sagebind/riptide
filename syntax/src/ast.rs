//! Abstract syntax tree definitions for the language syntax.

use crate::source::Span;
use std::fmt;

/// A function block, containing a list of pipelines to execute.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Block {
    /// Where in the source the block is defined.
    #[serde(skip)]
    pub span: Option<Span>,

    /// A list of named parameters.
    pub named_params: Option<Vec<String>>,

    /// An optional, final named parameter that receives unbound arguments as a
    /// list.
    pub vararg_param: Option<String>,

    /// A list of statements to execute.
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Statement {
    Assignment(AssignmentStatement),
    Import(ImportStatement),
    Pipeline(Pipeline),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssignmentStatement {
    pub target: AssignmentTarget,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImportStatement {
    pub path: String,
    pub imports: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssignmentTarget {
    /// Assign a value to the member of an object.
    MemberAccess(MemberAccess),

    /// Assign a variable.
    Variable(String),
}

/// A pipeline of function calls.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pipeline(pub Vec<Call>);

/// A function call.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Call {
    /// A function call for a named function.
    #[cfg_attr(feature = "serde", serde(rename = "NamedCall"))]
    Named {
        function: String,
        args: Vec<CallArg>,
    },

    /// A function call on a callable object.
    #[cfg_attr(feature = "serde", serde(rename = "UnnamedCall"))]
    Unnamed {
        function: Box<Expr>,
        args: Vec<CallArg>
    },
}

/// An argument to a function call.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CallArg {
    /// A single expression.
    Expr(Expr),

    /// A splat, expanding the expression as a list into multiple args.
    Splat(Expr),
}

/// Abstract representation of an expression.
///
/// Contains a variant for each different expression type.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged),
)]
pub enum Expr {
    Block(Block),
    Pipeline(Pipeline),
    MemberAccess(MemberAccess),
    CvarReference(CvarReference),
    CvarScope(CvarScope),
    Regex(RegexLiteral),
    Substitution(Substitution),
    Table(TableLiteral),
    List(ListLiteral),
    Number(f64),
    InterpolatedString(InterpolatedString),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemberAccess(pub Box<Expr>, pub String);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CvarReference(pub String);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CvarScope {
    pub name: CvarReference,
    pub value: Box<Expr>,
    pub scope: Block,
}

/// Value substitution.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Substitution {
    /// A format substitution with a variable and parameters, such as `${foo:.2}`.
    ///
    /// This always evaluates to a string, unless the referenced variable is not defined.
    Format(String, Option<String>),

    /// A pipeline substitution, such as `$(add 1 2 3)`.
    ///
    /// Evaluates to the final return value after executing the pipeline.
    Pipeline(Pipeline),

    /// A simple variable substitution, such as `$foo`.
    ///
    /// This gets evaluated to the current value of the variable identified.
    Variable(String),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableLiteral(pub Vec<TableEntry>);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableEntry {
    pub key: Expr,
    pub value: Expr
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListLiteral(pub Vec<Expr>);

/// An interpolated string literal.
///
/// An interpolated string is made up of a sequence of parts that, when stringified and concatenated in order, form the
/// desired string value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InterpolatedString(pub Vec<InterpolatedStringPart>);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged),
)]
pub enum InterpolatedStringPart {
    String(String),
    Substitution(Substitution),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RegexLiteral(pub String);

impl fmt::Display for RegexLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
