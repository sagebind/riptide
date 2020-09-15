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

    /// The name of the closure, if any. Since closures are naturally
    /// anonymous, names are optionally derived from the first binding to the
    /// closure in source code.
    pub(crate) name: Option<String>,
}

impl Closure {
    /// Get the name of the closure.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return a copy of this closure with the given name assigned.
    pub(crate) fn with_name(&self, name: String) -> Self {
        Self {
            block: self.block.clone(),
            scope: self.scope.clone(),
            name: Some(name),
        }
    }
}

impl PartialEq for Closure {
    fn eq(&self, rhs: &Closure) -> bool {
        ptr::eq(self, rhs)
    }
}
