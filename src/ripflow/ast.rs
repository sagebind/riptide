//! Definition and structures for the abstract syntax tree.
//!
//! Interestingly, the AST of a program is also the in-memory representation of
//! the program as it runs in the interpreter as well.
use std::borrow::Cow;


/// A node in an AST.
///
/// Contains a variant for each different node type.
pub enum Node {
    Nil,
    String(Cow<'static, str>),
    List(Vec<Node>),
    Block,
}

impl<S> From<S> for Node where S: Into<Cow<'static, str>> {
    fn from(value: S) -> Self {
        Node::String(value.into())
    }
}


/// A plain string.
pub struct String {
    value: Cow<'static, str>,
}


pub struct Block {}
