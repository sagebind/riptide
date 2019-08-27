/// Convenience macro for creating a table.
#[macro_export]
macro_rules! table {
    () => {
        $crate::table::Table::default()
    };

    (
        $(
            $key:expr => $value:expr,
        )*
    ) => {
        {
            let table = table!();
            $(
                table.set($key, $crate::value::Value::from($value));
            )*
            table
        }
    };
}

/// Convenience macro for throwing a runtime exception.
#[macro_export]
macro_rules! throw {
    ($($arg:tt)*) => {
        return Err($crate::exceptions::Exception::from(format!($($arg)*)))
    };
}

mod builtins;
pub mod closure;
pub mod exceptions;
mod foreign;
pub mod modules;
mod pipeline;
pub mod process;
pub mod runtime;
pub mod stdlib;
pub mod string;
pub mod table;
pub mod value;

// Re-export syntax crate.
pub mod syntax {
    pub use riptide_syntax::*;
}

pub mod prelude {
    pub use crate::exceptions::Exception;
    pub use crate::runtime::Runtime;
    pub use crate::table::Table;
    pub use crate::value::Value;
}

pub use crate::runtime::Runtime;
