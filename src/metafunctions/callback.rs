use crate::{Extractor, http::{Response, Request}};
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

/// Pipeline type, contains either a layer or the core of the pipeline
pub enum Pipeline {
    Layer(Arc<LayerFn>, Box<Pipeline>),
    Core(Arc<CoreFn>)
}

impl Pipeline {
    pub fn execute(self, s: Request) ->  Pin<Box<dyn Future<Output = Response> + Send>> {
        match self {
            Pipeline::Layer(func, pipeline_layer) => func(s, pipeline_layer),
            Pipeline::Core(core_fn) => core_fn(s)
        }.boxed()
    }
}

/// Type for the core handlers, that is, the ones that actually create a response
pub type CoreFn = Box<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;
/// Type representing middleware functions
pub type LayerFn = Box<dyn Fn(Request, Box<Pipeline>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;

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