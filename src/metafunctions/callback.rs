use crate::{Extractor, http::{Response, Request}};
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;

/// Callback trait, for http callbacks
pub trait Callback<A> {
    /// The invoke method should give back a pinned boxed future
    fn invoke(&self, args: A) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

/*
impl<T, F> Callback<()> for T where T: Fn() -> F + Sync, F: std::future::Future<Output = Response> + std::marker::Send {
    fn invoke(&self, _req: &Request) -> Pin<Box<dyn Future<Output = Response>>> {
        self().boxed()
    }
}

impl<T, F, A: Extractor> Callback<(A,)> for T where T: Fn(A) -> F + Sync, F: std::future::Future<Output = Response> + std::marker::Send {
    fn invoke(&self, req: &Request) -> Pin<Box<dyn Future<Output = Response>>> {
        let val = <A as Extractor>::extract(req);
        self(val).boxed()
    }
}
*/

impl<F, R> Callback<()> for F where F: Fn() -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static{
    fn invoke(&self, _args: ()) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self().boxed()
    }
}

impl<F, R, A> Callback<(A,)> for F where F: Fn(A) -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static, A: Extractor {
    fn invoke(&self, args: (A,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self(args.0).boxed()
    }
}

/*
#[async_trait::async_trait]
impl<F, T, E> Processor for T where
    T: Fn(E) -> F + Sync,
    E: Extractor,
    F: std::future::Future<Output = Response> + std::marker::Send {
    async fn handle(&self, req: &Request) -> Response {
        let value = <E as Extractor>::extract(req);
        self(value).await
    }
}
*/

/*
/// FromRequest trait impl for tuples
macro_rules! implement_processor ($(($n:tt, $T:ident)),+)) => {
        impl<Func, $($T,)+ Res> Handler<($($T,)+), Res> for Func
        where Func: Fn($($T,)+) -> Res + Clone + 'static,
                Res: Future,
                Res::Output: Responder,
        {
            fn call(&self, param: ($($T,)+)) -> Res {
                (self)($(param.$n,)+)
            }
        }
    }
};
impl<Func, A, B, C, Res> Handler<(A, B, C), Res> for Func
where Func: Fn(A, B, C) -> Res + Clone + 'static,
        Res: Future,
        Res::Output: Responder,
{
    fn call(&self, param: (A, B, C)) -> Res {
        (self)(param.0, param.1, param.2)
    }
}

*/