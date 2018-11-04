extern crate bytes;
extern crate fnv;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate nix;
extern crate riptide_syntax;
extern crate utf8;

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
                table.set($key, $value);
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
pub mod exceptions;
pub mod fd;
pub mod modules;
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
    pub use exceptions::Exception;
    pub use runtime::Runtime;
    pub use table::Table;
    pub use value::Value;
}
