//! Abstract syntax tree.

/// A function block, containing a list of pipelines to execute.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// A list of named parameters.
    pub named_params: Option<Vec<String>>,

    /// A list of statements to execute.
    pub statements: Vec<Pipeline>,
}

/// A pipeline of function calls.
#[derive(Clone, Debug, PartialEq)]
pub struct Pipeline {
    pub items: Vec<Call>,
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
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Substitution(Substitution),
    ExpandableString(String),
    String(String),
    Block(Block),
    Pipeline(Pipeline),
}

/// A variable substitution expression.
#[derive(Clone, Debug, PartialEq)]
pub struct Substitution {
    pub path: Vec<String>,
}
