use super::scope::Scope;
use gc::Gc;
use riptide_syntax::ast;
use std::ptr;

#[derive(Clone, Debug, gc::Finalize, gc::Trace)]
pub struct Closure {
    /// The AST of the chunk of code to be executed.
    #[unsafe_ignore_trace]
    pub(crate) block: ast::Block,

    /// The local scope the closure is defined in. May be `None` if the closure
    /// was compiled in a bare environment.
    pub(crate) scope: Option<Gc<Scope>>,
}

impl PartialEq for Closure {
    fn eq(&self, rhs: &Closure) -> bool {
        ptr::eq(self, rhs)
    }
}
