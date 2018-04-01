//! Abstract syntax tree.

/// A function block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    /// A list of named parameters.
    pub named_params: Option<Vec<String>>,

    /// A list of statements to execute.
    pub statements: Vec<Pipeline>,
}

/// A pipeline of function calls.
pub struct Pipeline {
    pub items: Vec<Call>,
}

/// A function call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    /// The function to invoke. Could be a binding name or a block.
    pub function: Box<Expr>,

    /// A list of argument expressions to pass to the function.
    pub args: Vec<Expr>,
}

/// Abstract representation of an expression.
///
/// Contains a variant for each different expression type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A string literal that may have variable expansions in it.
    ExpandableString(String),

    /// A string literal.
    String(String),

    /// A function call.
    Call(Call),

    /// A function block, containing a list of expressions to execute.
    Block(Block),
}
