use crate::{CoreFn, LayerFn, Pipeline, Callback, Extractor, http::{Request, Response, Method, MethodHandler}};
use std::collections::HashMap;
use futures::future::FutureExt;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

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
    pub(crate) method_callbacks: HashMap<Method, Arc<CoreFn>>,
    /// Default method to invoke, in case no branch gets a match
    pub(crate) default_callback: Option<Arc<CoreFn>>,
    /// Default method to invoke when no method was found, but the path exists
    pub(crate) default_method_callback: Option<Arc<CoreFn>>,
    /// Inner branches of the path
    pub(crate) branches: Vec<Path>,
    // /// Contains the middlewares that apply to this node and all the branches that depend on it
    pub(crate) layer_functions: Vec<Arc<LayerFn>>
}

impl Path {
    /// Creates a new path, represented by the passed string.
    ///
    /// ```
    /// let path = Path::new("/hello/world");
    /// ```
    ///
    /// That is valid syntax, but you will normally call other functions before storing the path in a variable.
    pub fn new<T: Into<String>>(path_string: T) -> Path {
        let mut tokenized_path: Vec<_> = path_string.into().split("/").map(|v| v.to_string()).collect();
        tokenized_path.retain(|v| v.len() != 0);
        Path {
            tokenized_path: tokenized_path,
            method_callbacks: HashMap::new(),
            default_callback: None,
            default_method_callback: None,
            branches: Vec::new(),
            layer_functions: Vec::new()
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
    pub fn with(mut self, method_callback: MethodHandler) -> Self {
        self.method_callbacks.insert(method_callback.method, Arc::new(method_callback.handler));
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
    pub fn defaults_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, callback: F) -> Self {
        self.default_callback = Some(Arc::new(Box::new(move |req: Request|  {
            let args = <A as Extractor>::extract(&req);
            callback.invoke(args).boxed()
        })));
        self
    }

    /// Adds a default method responder, in case no specific handler is found for the requested method.
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
    pub fn unmatched_method_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, callback: F) -> Self {
        self.default_method_callback = Some(Arc::new(Box::new(move |req: Request|  {
            let args = <A as Extractor>::extract(&req);
            callback.invoke(args).boxed()
        })));
        self
    }

    /// Adds a processing layer to the callbacks contained in this branch
    ///
    /// A layer is what is commonly known as middleware. The passed layer methods act as a wrap to the core handling functions of this path. It is important to note that layer functions have a very specific structure: each one receives a [`Request`](crate::http::Request) and a boxed [`Pipeline`](crate::Pipeline). The function must return a pinned boxed future. A Timing Layer/Middleware function is provided as an example
    ///
    /// ```
    /// let path = Path::new("/hello")
    ///     .with(Method::Get.to(|| async {Response::ok().body("¡Hola!")}))
    ///     .layer(|req: Request, pipeline: Box<Pipeline>| async {
    ///         // Example of timing layer / middleware
    ///         let now = std::time::Instant::now();
    ///         // Execute the deeper layers of the pipeline, passing the request
    ///         let response = pipeline.execute(req).await;
    ///         // Measure and print time
    ///         let elapsed = now.elapsed().as_nanos();
    ///         info!("Process time: {} ns", elapsed);
    ///         // We return the request for further possible processing.
    ///         request
    /// }.boxed());
    /// ```
    ///
    /// Calling the function multiple times will wrap the preceeding layer (or core handlers), like an onion 🧅.
    pub fn layer<F: Fn(Request, Box<Pipeline>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static>(mut self, layer_fn: F) -> Self {
        self.layer_functions.push(Arc::new(Box::new(layer_fn)));
        self
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut s = format!("> path: /{}", self.tokenized_path.join("/"));
        for (method, _) in &self.method_callbacks {
            s += &format!("\n> Method {} has custom reply", method.to_str());
        }
        s += &format!("\n> Node has {} layer(s)", self.layer_functions.len());
        // Now each branch
        for branch in &self.branches {
            s += &format!("\n{}", branch).replace("\n", "\n--");
        }
        write!(formatter, "{}", s)
    }
}

//let path = Path::new("/hello").replies(Method::Get.with(my_function));