use crate::{WrappedHandler, Callback, Extractor, http::{Request, Method, MethodHandler}};
use std::collections::HashMap;
use futures::future::FutureExt;

/// Main building block for cataclysm
///
/// The Path structure is meant to construct a __tree__ of possible paths that http calls can follow in order to give out a response
/// ```
/// let server_path = Path::new("/").with(Method::Get.to(index));
/// ```
pub struct Path {
    /// Tokenized path. An empty vec means it replies to the root
    pub(crate) tokenized_path: Vec<String>,
    /// Map of functions
    pub(crate) method_handlers: HashMap<Method, WrappedHandler>,
    /// Default method to invoke, in case no branch gets a match
    pub(crate) default_method: Option<WrappedHandler>,
    /// Inner branches of the path
    pub(crate) branches: Vec<Path>
}

impl Path {
    pub fn new<T: Into<String>>(path_string: T) -> Path {
        let mut tokenized_path: Vec<_> = path_string.into().split("/").map(|v| v.to_string()).collect();
        tokenized_path.retain(|v| v.len() != 0);
        Path {
            tokenized_path: tokenized_path,
            method_handlers: HashMap::new(),
            default_method: None,
            branches: Vec::new()
        }
    }

    /// Adds a callback to a method or a group of methods
    ///
    /// This function is the main building block for callbacks in the path tree
    ///
    /// ```
    /// let server = Server::builder(
    ///     Path::new("/scope").with(Method::Get.to(index))
    /// ).build();
    /// ```
    pub fn with(mut self, method_handler: MethodHandler) -> Self {
        self.method_handlers.insert(method_handler.method, method_handler.handler);
        self
    }

    /// Adds a nested path to the actual path
    ///
    /// Usefull when you try to define scopes or so
    ///
    /// ```
    /// let server = Server::builder(Path::new("/scope")
    ///     .nested(Path::new("/index")
    ///         // Method Get at /scope/index replies with index
    ///         .with(Method::Get.to(index))
    ///     )
    /// ).build();
    /// ```
    pub fn nested(mut self, path: Path) -> Self {
        self.branches.push(path);
        self
    }

    /// Adds a default path, in case of no nested matching.
    ///
    /// ```
    /// let server = Server::builder(
    ///     Path::new("/").defaults_to(|| async {
    ///         Response::ok().body("Are you lost?")
    ///     })
    /// ).build();
    /// ```
    pub fn defaults_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, handler: F) -> Self {
        self.default_method = Some(Box::new(move |req: &Request|  {
            let args = <A as Extractor>::extract(req);
            handler.invoke(args).boxed()
        }));
        self
    }

    /// Adds a default method responder, in case no specific handler is found.
    ///
    /// By default, unmatched methods reply with a `405 Method Not Allowed`, but this function allows override of such behaviour.
    ///
    /// ```
    /// let server = Server::builder(
    ///     Path::new("/").with(Method::Get.to(|| async {
    ///         Response::ok().body("Supported!")
    ///     })).unmatched_method_to(|| async {
    ///         Response::ok().body("Unsupported, please try with GET")
    ///     })
    /// ).build();
    /// ```
    pub fn unmatched_method_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, handler: F) -> Self {
        /*
        self.default_method = Some(Box::new(move |req: &Request|  {
            let args = <A as Extractor>::extract(req);
            handler.invoke(args).boxed()
        }));
        */
        self
    }
}

//let path = Path::new("/hello").replies(Method::Get.with(my_function));