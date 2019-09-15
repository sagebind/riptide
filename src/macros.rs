/// Convenience macro for creating a table.
#[macro_export]
macro_rules! table {
    () => {
        $crate::runtime::table::Table::default()
    };

    (
        $(
            $key:expr => $value:expr,
        )*
    ) => {
        {
            let table = table!();
            $(
                table.set($key, $crate::runtime::value::Value::from($value));
            )*
            table
        }
    };
}

/// Convenience macro for throwing a runtime exception.
#[macro_export]
macro_rules! throw {
    ($($arg:tt)*) => {
        return Err($crate::runtime::exceptions::Exception::from(format!($($arg)*)))
    };
}
