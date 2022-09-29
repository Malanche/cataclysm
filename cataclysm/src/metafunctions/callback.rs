use crate::{
    additional::Additional,
    http::{Response, Request}
};
#[cfg(feature = "ws")]
use crate::ws::{WebSocketWriter, WebSocketReader};
#[cfg(feature = "demon")]
use apocalypse::Gate;
#[cfg(feature = "demon")]
use tokio::net::tcp::OwnedReadHalf;
#[cfg(feature = "demon")]
use crate::Error;
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

/// Pipeline type, contains either a layer or the core of the pipeline
pub enum Pipeline<T> {
    /// Processing layer
    Layer(Arc<LayerFn<T>>, Box<Pipeline<T>>),
    /// Core layer
    Core(Arc<CoreFn<T>>)
}

impl<T> Pipeline<T> {
    pub fn execute(self, s: Request, a: Arc<Additional<T>>) ->  Pin<Box<dyn Future<Output = Response> + Send>> {
        match self {
            Pipeline::Layer(func, pipeline_layer) => func(s, pipeline_layer, a),
            Pipeline::Core(core_fn) => core_fn(s, a)
        }.boxed()
    }
}

/// Type for the core handlers, that is, the ones that actually create a response
pub type CoreFn<T> = Box<dyn Fn(Request, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;
/// Type representing middleware functions
pub type LayerFn<T> = Box<dyn Fn(Request, Box<Pipeline<T>>, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;
/// Type representing a websocket handler function
#[cfg(feature = "ws")]
pub type WebSocketFn = Box<dyn Fn(WebSocketWriter) -> Pin<Box<dyn Future<Output = Box<dyn WebSocketReader>> + Send>> + Send + Sync>;
/// Type representing a websocket-demon handler function
#[cfg(feature = "demon")]
pub type WebSocketDemonFn = Box<dyn Fn(WebSocketWriter, Gate, OwnedReadHalf) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> + Send + Sync>;

/// Callback trait, for http callbacks
pub trait Callback<A> {
    /// The invoke method should give back a pinned boxed future
    fn invoke(&self, args: A) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

// Callback implementation for empty tupple
impl<F, R> Callback<()> for F where F: Fn() -> R, R: Future<Output = Response> + Send + 'static{
    fn invoke(&self, _args: ()) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self().boxed()
    }
}

/// This macro implements the trait for a given indexed tuple
macro_rules! callback_for_many {
    ($struct_name:ident $index:tt) => {
        impl<F, R, $struct_name> Callback<($struct_name,)> for F where F: Fn($struct_name) -> R, R: Future<Output = Response> + Send + 'static {
            fn invoke(&self, args: ($struct_name,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self(args.$index).boxed()
            }
        }
    };
    ($($struct_name:ident $index:tt),+) => {
        impl<F, R, $($struct_name),+> Callback<($($struct_name),+)> for F where F: Fn($($struct_name),+) -> R, R: Future<Output = Response> + Send + 'static {
            fn invoke(&self, args: ($($struct_name),+)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self($(args.$index,)+).boxed()
            }
        }
    }
}

// We implement the trait for up to 5 arguments at the moment
callback_for_many!(A 0);
callback_for_many!(A 0, B 1);
callback_for_many!(A 0, B 1, C 2);
callback_for_many!(A 0, B 1, C 2, D 3);
callback_for_many!(A 0, B 1, C 2, D 3, E 4);