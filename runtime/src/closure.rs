use super::scope::Scope;
use riptide_syntax::ast;
use std::ptr;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Closure {
    /// The AST of the chunk of code to be executed.
    pub(crate) block: ast::Block,

    /// The local scope the closure is defined in.
    pub(crate) scope: Rc<Scope>,
}

impl PartialEq for Closure {
    fn eq(&self, rhs: &Closure) -> bool {
        ptr::eq(self, rhs)
    }
}
