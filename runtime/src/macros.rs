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

/// Helpful macro for generating a heap-allocated foreign function from a
/// closure that takes care of lifetime fiddly bits for you.
#[macro_export]
macro_rules! foreign_fn {
    ($(clone $clone:ident),* |$fiber:pat, $args:ident| $block:expr) => {{
        type LocalBoxFuture<'a, T> = ::std::pin::Pin<Box<dyn ::std::future::Future<Output = T> + 'a>>;

        fn constrain<F>(f: F) -> F
        where
            F: for<'a> Fn(&'a mut $crate::Fiber, Vec<$crate::Value>) -> LocalBoxFuture<'a, Result<$crate::Value, $crate::Exception>>,
        {
            f
        }

        let closure = constrain(move |$fiber, $args| {
            $(
                let $clone = $clone.clone();
            )*
            Box::pin(async move {
                $block
            })
        });

        $crate::ForeignFn::from(closure)
    }};
}
