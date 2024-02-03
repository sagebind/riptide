//! Abstract syntax tree definitions for the language syntax.

use crate::source::Span;
use regex::bytes::Regex;
use std::fmt;

macro_rules! derive_debug_enum_transparent {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $variant:ident($inner:ty)
                $(,)*
            )*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $variant($inner),
            )*
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $(
                        $name::$variant(ref inner) => ::std::fmt::Debug::fmt(inner, f),
                    )*
                }
            }
        }
    };
}

/// A function block, containing a list of pipelines to execute.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// Where in the source the block is defined.
    pub span: Option<Span>,

    /// A list of named parameters.
    pub named_params: Option<Vec<String>>,

    /// An optional, final named parameter that receives unbound arguments as a
    /// list.
    pub vararg_param: Option<String>,

    /// A list of statements to execute.
    pub statements: Vec<Statement>,
}

/// A subroutine is a named block.
#[derive(Clone, Debug, PartialEq)]
pub struct Subroutine {
    pub name: String,
    pub block: Block,
}

derive_debug_enum_transparent! {
    #[derive(Clone, PartialEq)]
    pub enum Statement {
        Assignment(AssignmentStatement),
        Import(ImportStatement),
        Pipeline(Pipeline),
        Return(Option<Expr>),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignmentStatement {
    pub target: AssignmentTarget,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportStatement {
    pub path: String,
    pub clause: ImportClause,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportClause {
    Items(Vec<String>),
    Wildcard,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssignmentTarget {
    /// Assign a value to the member of an object.
    MemberAccess(MemberAccess),

    /// Assign a variable.
    Variable(String),
}

/// A pipeline of function calls.
#[derive(Clone, Debug, PartialEq)]
pub struct Pipeline(pub Vec<Call>);

/// A function call.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// A function call for a named function.
    Named {
        function: String,
        args: Vec<CallArg>,
    },

    /// A function call on a callable object.
    Unnamed {
        function: Box<Expr>,
        args: Vec<CallArg>
    },
}

/// An argument to a function call.
#[derive(Clone, Debug, PartialEq)]
pub enum CallArg {
    /// A single expression.
    Expr(Expr),

    /// A splat, expanding the expression as a list into multiple args.
    Splat(Expr),
}

derive_debug_enum_transparent! {
    /// Abstract representation of an expression.
    ///
    /// Contains a variant for each different expression type.
    #[derive(Clone, PartialEq)]
    pub enum Expr {
        Block(Block),
        Subroutine(Subroutine),
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccess(pub Box<Expr>, pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct CvarReference(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct CvarScope {
    pub name: CvarReference,
    pub value: Box<Expr>,
    pub scope: Block,
}

/// Value substitution.
#[derive(Clone, Debug, PartialEq)]
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
pub struct TableLiteral(pub Vec<TableEntry>);

#[derive(Clone, Debug, PartialEq)]
pub struct TableEntry {
    pub key: Expr,
    pub value: Expr
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListLiteral(pub Vec<Expr>);

/// An interpolated string literal.
///
/// An interpolated string is made up of a sequence of parts that, when stringified and concatenated in order, form the
/// desired string value.
#[derive(Clone, Debug, PartialEq)]
pub struct InterpolatedString(pub Vec<InterpolatedStringPart>);

#[derive(Clone, Debug, PartialEq)]
pub enum InterpolatedStringPart {
    String(String),
    Substitution(Substitution),
}

/// A regular expression literal.
///
/// Regular expressions written in source code are always validated and parsed
/// as part of the syntax parsing routine, and converted into a regular
/// expression AST as part of the overall program AST.
///
/// This also can be used as a runtime optimizations, as regex literals do not
/// have to be re-parsed every time they are used without any effort from the
/// user. They can be executed directly from AST memory.
#[derive(Clone, Debug)]
pub struct RegexLiteral(pub Regex);

impl fmt::Display for RegexLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for RegexLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}
