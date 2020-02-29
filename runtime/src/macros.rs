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
