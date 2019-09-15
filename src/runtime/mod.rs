mod modules;
mod runtime;
pub mod builtins;
pub mod closure;
pub mod exceptions;
pub mod foreign;
pub mod scope;
pub mod string;
pub mod table;
pub mod value;

// Re-export syntax crate.
pub mod syntax {
    pub use riptide_syntax::*;
}

pub mod prelude {
    pub use crate::runtime::exceptions::Exception;
    pub use crate::runtime::table::Table;
    pub use crate::runtime::value::Value;
    pub use crate::runtime::runtime::Runtime;
}

pub use self::runtime::Runtime;
