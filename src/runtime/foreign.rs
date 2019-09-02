use super::{
    exceptions::Exception,
    runtime::Runtime,
    value::Value,
};
use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// A native function that can be invoked by scripts through a runtime as well
/// as in native code.
#[derive(Clone)]
pub struct ForeignFn(Rc<dyn for<'a> Fn(&'a mut Runtime, &'a [Value]) -> LocalBoxFuture<'a, Result<Value, Exception>>>);

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

impl ForeignFn {
    pub async fn call(&self, runtime: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
        (&self.0)(runtime, args).await
    }
}

impl<F> From<F> for ForeignFn
where
    F: 'static + for<'a, 'b> AsyncFn2<&'a mut Runtime, &'b [Value], Output = Result<Value, Exception>>
{
    fn from(f: F) -> Self {
        ForeignFn(Rc::new(move |runtime, args| {
            f.call(runtime, args).boxed_local()
        }))
    }
}

// Workaround for https://github.com/rust-lang/rust/issues/51004
macro_rules! impl_async_fn {
    ($(($FnOnce:ident, $FnMut:ident, $Fn:ident, ($($arg:ident: $arg_ty:ident,)*)),)*) => {
        $(
            pub trait $FnOnce<$($arg_ty,)*> {
                type Output;
                type Future: Future<Output = Self::Output>;
                fn call_once(self, $($arg: $arg_ty,)*) -> Self::Future;
            }
            pub trait $FnMut<$($arg_ty,)*>: $FnOnce<$($arg_ty,)*> {
                fn call_mut(&mut self, $($arg: $arg_ty,)*) -> Self::Future;
            }
            pub trait $Fn<$($arg_ty,)*>: $FnMut<$($arg_ty,)*> {
                fn call(&self, $($arg: $arg_ty,)*) -> Self::Future;
            }
            impl<$($arg_ty,)* F, Fut> $FnOnce<$($arg_ty,)*> for F
            where
                F: FnOnce($($arg_ty,)*) -> Fut,
                Fut: Future,
            {
                type Output = Fut::Output;
                type Future = Fut;
                fn call_once(self, $($arg: $arg_ty,)*) -> Self::Future {
                    self($($arg,)*)
                }
            }
            impl<$($arg_ty,)* F, Fut> $FnMut<$($arg_ty,)*> for F
            where
                F: FnMut($($arg_ty,)*) -> Fut,
                Fut: Future,
            {
                fn call_mut(&mut self, $($arg: $arg_ty,)*) -> Self::Future {
                    self($($arg,)*)
                }
            }
            impl<$($arg_ty,)* F, Fut> $Fn<$($arg_ty,)*> for F
            where
                F: Fn($($arg_ty,)*) -> Fut,
                Fut: Future,
            {
                fn call(&self, $($arg: $arg_ty,)*) -> Self::Future {
                    self($($arg,)*)
                }
            }
        )*
    }
}

impl_async_fn! {
    (AsyncFnOnce0, AsyncFnMut0, AsyncFn0, ()),
    (AsyncFnOnce1, AsyncFnMut1, AsyncFn1, (a0:A0, )),
    (AsyncFnOnce2, AsyncFnMut2, AsyncFn2, (a0:A0, a1:A1, )),
}
