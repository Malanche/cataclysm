use crate::{Extractor, http::{Response, Request}};
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;

/// Short for the function signature that wraps the handlers
pub type WrappedHandler = Box<dyn Fn(&Request) -> Pin<Box<dyn futures::Future<Output = Response> + Send>> + Send + Sync>;

/// Callback trait, for http callbacks
pub trait Callback<A> {
    /// The invoke method should give back a pinned boxed future
    fn invoke(&self, args: A) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

// Callback implementation for empty tupple
impl<F, R> Callback<()> for F where F: Fn() -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static{
    fn invoke(&self, _args: ()) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self().boxed()
    }
}

/// This macro implements the trait for a given indexed tuple
macro_rules! callback_for_many {
    ($struct_name:ident $index:tt) => {
        impl<F, R, $struct_name> Callback<($struct_name,)> for F where F: Fn($struct_name) -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static, $struct_name: Extractor {
            fn invoke(&self, args: ($struct_name,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self(args.$index).boxed()
            }
        }
    };
    ($($struct_name:ident $index:tt),+) => {
        impl<F, R, $($struct_name),+> Callback<($($struct_name),+)> for F where F: Fn($($struct_name),+) -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static, $($struct_name: Extractor),+ {
            fn invoke(&self, args: ($($struct_name),+)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self($(args.$index,)+).boxed()
            }
        }
    }
}

// Here we are
callback_for_many!(A 0);
callback_for_many!(A 0, B 1);
callback_for_many!(A 0, B 1, C 2);
callback_for_many!(A 0, B 1, C 2, D 3);
callback_for_many!(A 0, B 1, C 2, D 3, E 4);