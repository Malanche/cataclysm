use crate::{
    additional::Additional,
    http::{Response, Request}
};
#[cfg(feature = "stream")]
use crate::Stream;
use futures::future::FutureExt;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

pub(crate) struct PipelineInfo<T> {
    /// Contains information about how the handler function was found
    #[cfg(feature = "full_log")]
    pub pipeline_track: PipelineTrack,
    pub pipeline_kind: PipelineKind<T>
}

/// Contains information about the callback
#[cfg(feature = "full_log")]
#[derive(Debug, Clone)]
pub(crate) enum PipelineTrack {
    Exact(String),
    UnmatchedMethod(String),
    File(String),
    Default(String),
    #[cfg(feature = "stream")]
    Stream(String)
}

#[cfg(feature = "full_log")]
impl PipelineTrack {
    #[cfg(feature = "full_log")]
    pub(crate) fn preconcat<A: AsRef<str>>(&mut self, token: A) {
        match self {
            PipelineTrack::Exact(s) | PipelineTrack::UnmatchedMethod(s) | PipelineTrack::File(s) | PipelineTrack::Default(s) => {
                if s.is_empty() {
                    *s = token.as_ref().to_string();
                } else {
                    *s = format!("{}/{}", token.as_ref(), s);
                }
            },
            #[cfg(feature = "stream")]
            PipelineTrack::Stream(s) => {
                if s.is_empty() {
                    *s = token.as_ref().to_string();
                } else {
                    *s = format!("{}/{}", token.as_ref(), s);
                }
            }
        }
    }
}

#[cfg(feature = "full_log")]
impl std::fmt::Display for PipelineTrack {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            PipelineTrack::Exact(s) => format!("Exact({})", s),
            PipelineTrack::UnmatchedMethod(s) => format!("UnmatchedMethod({})", s),
            PipelineTrack::File(s) => format!("File({})", s),
            PipelineTrack::Default(s) => format!("Default({})", s),
            #[cfg(feature = "stream")]
            PipelineTrack::Stream(s) => format!("Stream({})", s)
        };
        write!(formatter, "{}", content)
    } 
}

/// Wrapper pipeline for the server to work with
pub(crate) enum PipelineKind<T> {
    NormalPipeline{
        pipeline: Pipeline<T>
    },
    #[cfg(feature = "stream")]
    StreamPipeline{
        pipeline: Arc<HandlerFn<T>>
    }
}

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

/// Callback trait, for http callbacks
pub trait Callback<A> {
    /// The invoke method should give back a pinned boxed future
    fn invoke(&self, args: A) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

// Callback implementation for empty tupple
impl<F, R, Z: Into<Response>> Callback<()> for F where F: Fn() -> R, R: Future<Output = Z> + Send + 'static{
    fn invoke(&self, _args: ()) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
        self().map(|v| v.into()).boxed()
    }
}

/// This macro implements the trait for a given indexed tuple
macro_rules! callback_for_many {
    ($struct_name:ident $index:tt) => {
        impl<K, R, Z: Into<Response>, $struct_name> Callback<($struct_name,)> for K where K: Fn($struct_name) -> R, R: Future<Output = Z> + Send + 'static {
            fn invoke(&self, args: ($struct_name,)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self(args.$index).map(|v| v.into()).boxed()
            }
        }
    };
    ($($struct_name:ident $index:tt),+) => {
        impl<K, R, Z: Into<Response>, $($struct_name),+> Callback<($($struct_name),+)> for K where K: Fn($($struct_name),+) -> R, R: Future<Output = Z> + Send + 'static {
            fn invoke(&self, args: ($($struct_name),+)) -> Pin<Box<dyn Future<Output = Response>  + Send>> {
                self($(args.$index,)+).map(|v| v.into()).boxed()
            }
        }
    }
}

// We implement the trait for up to 8 arguments at the moment
callback_for_many!(A 0);
callback_for_many!(A 0, B 1);
callback_for_many!(A 0, B 1, C 2);
callback_for_many!(A 0, B 1, C 2, D 3);
callback_for_many!(A 0, B 1, C 2, D 3, E 4);
callback_for_many!(A 0, B 1, C 2, D 3, E 4, F 5);
callback_for_many!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
callback_for_many!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);


/// Type for the core handler, that is, as a [CoreFn](CoreFn) but can also take a stream (after valid http request processing)
#[cfg(feature = "stream")]
pub type HandlerFn<T> = Box<dyn Fn(Request, Arc<Additional<T>>, Stream) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// StreamCallback trait, similar to a normal http callback but receives the TcpStream
#[cfg(feature = "stream")]
pub trait StreamCallback<A> {
    fn invoke(&self, stream: Stream, args: A) -> Pin<Box<dyn Future<Output = ()>  + Send>>;
}

#[cfg(feature = "stream")]
impl<K, R> StreamCallback<()> for K where K: Fn(Stream) -> R, R: Future<Output = ()> + Send + 'static {
    fn invoke(&self, stream: Stream, _args: ()) -> Pin<Box<dyn Future<Output = ()>  + Send>> {
        self(stream).boxed()
    }
}

/// This macro implements the trait for a given indexed tuple
#[cfg(feature = "stream")]
macro_rules! stream_callback_for_many {
    ($struct_name:ident $index:tt) => {
        impl<K, R, $struct_name> StreamCallback<($struct_name, )> for K where K: Fn(Stream, $struct_name) -> R, R: Future<Output = ()> + Send + 'static {
            fn invoke(&self, stream: Stream, args: ($struct_name,)) -> Pin<Box<dyn Future<Output = ()>  + Send>> {
                self(stream, args.$index).boxed()
            }
        }
    };
    ($($struct_name:ident $index:tt),+) => {
        impl<K, R, $($struct_name),+> StreamCallback<($($struct_name),+)> for K where K: Fn(Stream, $($struct_name),+) -> R, R: Future<Output = ()> + Send + 'static {
            fn invoke(&self, stream: Stream, args: ($($struct_name),+)) -> Pin<Box<dyn Future<Output = ()>  + Send>> {
                self(stream, $(args.$index,)+).boxed()
            }
        }
    }
}

#[cfg(feature = "stream")]
stream_callback_for_many!(A 0);
#[cfg(feature = "stream")]
stream_callback_for_many!(A 0, B 1);
#[cfg(feature = "stream")]
stream_callback_for_many!(A 0, B 1, C 2);
#[cfg(feature = "stream")]
stream_callback_for_many!(A 0, B 1, C 2, D 3);
#[cfg(feature = "stream")]
stream_callback_for_many!(A 0, B 1, C 2, D 3, E 4);
#[cfg(feature = "stream")]
stream_callback_for_many!(A 0, B 1, C 2, D 3, E 4, F 5);