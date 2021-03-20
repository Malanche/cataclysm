use crate::{Extractor, http::Response};
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;

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

/*
macro_rules! callback_for_tuple {
    ($($param_names:ident),+) => {
        impl<F, R, $($param_names),+> Callback<($($param_names),+,)> for F where F: Fn($($param_names),+) -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static, $($param_names),+ {
            fn invoke(&self, args: (A,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self(args.0).boxed()
            }
        }
    }
}
*/

/*
// Now, macro to implement for tuples of up to 4 extractors
impl<F, R, A> Callback<(A,)> for F where F: Fn(A) -> R + Sync, R: Future<Output = Response> + Sync + Send + 'static, A: Extractor {
    fn invoke(&self, args: (A,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self(args.0).boxed()
    }
}
*/