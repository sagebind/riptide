//! Abstract syntax tree.


/// Abstract representation of an expression.
///
/// Contains a variant for each different expression type.
#[derive(Clone, Debug)]
pub enum Expr {
    /// A string literal that may have variable expansions in it.
    ExpandableString(String),

    /// A string literal.
    String(String),

    /// A function call.
    Call {
        /// The function to invoke. Could be a binding name or a block.
        function: Box<Expr>,

        /// A list of arguments to pass to the function.
        args: Vec<Expr>,
    },

    /// A function block, containing a list of expressions to execute.
    Block(Vec<Expr>),
}
