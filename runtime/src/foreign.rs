use crate::prelude::*;
use futures::future::{FutureExt, LocalBoxFuture};
use std::{
    future::Future,
    rc::Rc,
};

/// A native function that can be invoked by scripts through a runtime as well
/// as in native code.
#[derive(Clone)]
pub struct ForeignFn(Rc<dyn private::ForeignFnTrait>);

impl ForeignFn {
    pub(crate) fn new(f: impl private::ForeignFnTrait) -> Self {
        Self(Rc::new(f))
    }

    pub async fn call(&self, runtime: &mut Fiber, args: Vec<Value>) -> Result<Value, Exception> {
        (&self.0).call(runtime, args).await
    }
}

impl<F: private::ForeignFnTrait> From<F> for ForeignFn {
    fn from(f: F) -> Self {
        ForeignFn::new(f)
    }
}

mod private {
    use super::*;

    pub trait ForeignFnTrait: 'static {
        fn call<'a>(&self, fiber: &'a mut Fiber, args: Vec<Value>) -> LocalBoxFuture<'a, Result<Value, Exception>>;
    }

    impl<F> ForeignFnTrait for F
    where
        F: for<'a> AsyncFn2<&'a mut Fiber, Vec<Value>, Output = Result<Value, Exception>> + 'static
    {
        fn call<'a>(&self, fiber: &'a mut Fiber, args: Vec<Value>) -> LocalBoxFuture<'a, Result<Value, Exception>> {
            self.call(fiber, args).boxed_local()
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
        (AsyncFnOnce2, AsyncFnMut2, AsyncFn2, (a0:A0, a1:A1, )),
    }
}
