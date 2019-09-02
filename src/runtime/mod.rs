/// Convenience macro for creating a table.
#[macro_export]
macro_rules! table {
    () => {
        $crate::Table::default()
    };

    (
        $(
            $key:expr => $value:expr,
        )*
    ) => {
        {
            let table = table!();
            $(
                table.set($key, $crate::Value::from($value));
            )*
            table
        }
    };
}

/// Convenience macro for throwing a runtime exception.
#[macro_export]
macro_rules! throw {
    ($($arg:tt)*) => {
        return Err($crate::Exception::from(format!($($arg)*)))
    };
}

mod builtins;
mod closure;
mod exceptions;
mod foreign;
mod io;
mod modules;
mod pipes;
mod process;
mod reactor;
mod runtime;
mod scope;
mod stdlib;
mod string;
mod table;
mod value;

// Re-export syntax crate.
pub mod syntax {
    pub use riptide_syntax::*;
}

pub mod prelude {
    pub use crate::runtime::exceptions::Exception;
    pub use crate::runtime::runtime::Runtime;
    pub use crate::runtime::table::Table;
    pub use crate::runtime::value::Value;
}

pub use self::exceptions::Exception;
pub use self::foreign::ForeignFn;
pub use self::runtime::Runtime;
pub use self::table::Table;
pub use self::value::Value;
