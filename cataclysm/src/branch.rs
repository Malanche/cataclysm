use std::collections::{HashMap, HashSet};
use regex::Regex;
use futures::future::FutureExt;
use crate::{
    additional::Additional,
    CoreFn, LayerFn, Extractor, Callback, Pipeline,
    http::{Method, Request, Response, MethodHandler}
};
use crate::metafunctions::callback::{PipelineKind, PipelineInfo};
#[cfg(feature = "stream")]
use crate::{HandlerFn, StreamCallback, Stream};
#[cfg(feature = "full_log")]
use crate::metafunctions::callback::{PipelineTrack};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

enum BranchKind {
    Exact,
    Pattern,
    Default
}

/// ## Main cataclysm structure for route handling
///
/// Branches are cataclysm's main building block. It is a really simple pattern matching system, with the following priorities. They are named branches to avoid conflict with the [Path](crate::http::Path) extractor.
///
/// 1. Exact matching
/// 2. Pattern matching
/// 3. Default branches (a.k.a, variable handling in branches)
/// 4. Stream handler
///
/// In the case of exact matching, the path constructor is pretty straight forward
///
/// ```rust
/// # use cataclysm::Branch;
/// let branch: Branch<()> = Branch::new("/hello/world");
/// ```
///
/// Pattern matching is a bit more complex
///
/// ```rust
/// # use cataclysm::Branch;
/// // matches any route that starts with `/hello/` and then words of 3 or 4 letters, no numbers
/// let branch: Branch<()> = Branch::new("/hello/{regex:^[A-Za-z\\d]{3,4}$}");
/// ```
///
/// Last but not least, we have variable detection, with no regex
///
/// ```rust
/// # use cataclysm::Branch;
/// // matches any route that contains "/hello/{:variable}"
/// let branch: Branch<()> = Branch::new("/hello/{:variable}");
/// ```
///
/// There is an important thing to note about most methods of this structure. When you create a branch with multiple parts in the path, a tree gets spawned containing each token in the path, however, methods like `layer`, `nest`, and `with` operatoe on the top-level-token of the path, i.e., if you create a branch like this
///
/// ```rust,no_run
/// # use cataclysm::Branch;
/// let branch: Branch<()> = Branch::new("/path/with/many/tokens");
/// ```
///
/// And then you execute the `with` method, the callback you provide will reply only to `/path/with/many/tokens`. 
pub struct Branch<T> {
    /// Exact match branches
    exact_branches: HashMap<String, Branch<T>>,
    /// Regex match branches
    pattern_branches: Vec<(Regex, Branch<T>)>,
    /// Variable branch, only one per branch
    variable_branch: Option<(String, Box<Branch<T>>)>,
    /// Original source that created the branch, to point to the top node
    source: String,
    /// Method Callbacks
    method_callbacks: HashMap<Method, Arc<CoreFn<T>>>,
    /// Default method callback
    default_method_callback: Option<Arc<CoreFn<T>>>,
    /// Default callback for this node, and all the non-matching children
    default_callback: Option<Arc<CoreFn<T>>>,
    /// File callback, in case this endpoint wants to be used for static file serving
    files_callback: Option<Arc<CoreFn<T>>>,
    /// Layer functions on this branch
    layers: Vec<Arc<LayerFn<T>>>,
    /// Stream handler, when no other match was found
    #[cfg(feature = "stream")]
    stream_handler: Option<Arc<HandlerFn<T>>>
}

impl<T> std::fmt::Display for Branch<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut content = String::new();
        for (branch_id, branch) in self.exact_branches.iter() {
            content += &format!("\n--> {}", branch_id);
            let remaining_content = format!("{}", branch);
            if !remaining_content.is_empty() {
                content += &format!("\n{}", remaining_content).replace("-->", "---->");
            }
        }
        for (pattern, branch) in self.pattern_branches.iter() {
            content += &format!("\n--> :regex {}", pattern.as_str());
            let remaining_content = format!("{}", branch);
            if !remaining_content.is_empty() {
                content += &format!("\n{}", remaining_content).replace("-->", "---->");
            }
        }
        if let Some((var_id, variable_branch)) = &self.variable_branch {
            content += &format!("\n--> :variable_branch ({}):", var_id);
            let remaining_content = format!("{}", variable_branch);
            if !remaining_content.is_empty() {
                content += &format!("\n{}", remaining_content).replace("-->", "---->");
            }
        }
        write!(formatter, "{}", content.trim_start())
    }
}

impl<T: Sync + Send> Branch<T> {
    /// Creates a new branch
    ///
    /// ```rust
    /// # use cataclysm::Branch;
    /// let branch: Branch<()> = Branch::new("/hello/world");
    /// ```
    pub fn new<A: AsRef<str>>(trail: A) -> Branch<T> {
        // Tokenizamos la cadena
        let trimmed_trail = trail.as_ref().trim_start_matches("/");
        let mut branch = Branch {
            exact_branches: HashMap::new(),
            pattern_branches: vec![],
            variable_branch: None,
            source: trail.as_ref().to_string(),
            method_callbacks: HashMap::new(),
            default_method_callback: None,
            default_callback: None,
            files_callback: None,
            #[cfg(feature = "stream")]
            stream_handler: None,
            layers: vec![]
        };
        let (base, rest_branch) = if let Some((base, rest)) = trimmed_trail.tokenize_once() {
            let rest_branch = Branch::new(rest);
            (base.to_string(), rest_branch)
        } else {
            // Si el único token tiene longitud != 0, añadimos un branch.
            if !trimmed_trail.is_empty() {
                (trimmed_trail.to_string(), Branch::new(""))
            } else {
                // We return immediately with no modifications to the branch
                return branch;
            }
        };

        match Branch::<T>::clasify(&base) {
            BranchKind::Exact => {branch.exact_branches.insert(String::from(base), rest_branch);},
            BranchKind::Pattern => branch.pattern_branches.push((Regex::new(base.trim_start_matches("{regex:").trim_end_matches("}")).unwrap(), rest_branch)),
            BranchKind::Default => branch.variable_branch = Some((base.trim_start_matches("{:").trim_end_matches("}").to_string(), Box::new(rest_branch)))
        };

        // This might look inefficient, but it will only run during branch building
        branch
    }

    /// Adds a callback to a branch
    ///
    /// This function is the main building block for callbacks in the branch. A [MethodHandler](crate::http::MethodHandler) consists of a Method, and a callback function. Se the [Method](crate::http::Method) structure to see how to construct them.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Method, Response}};
    /// // Example index function
    /// async fn index() -> Response {
    ///     Response::ok().body("hello")
    /// }
    ///
    /// // Branch that will reply to a get method in `/scope`
    /// let branch: Branch<()> = Branch::new("/scope").with(Method::Get.to(index));
    /// ```
    pub fn with(mut self, method_callback: MethodHandler<T>) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        let handler = Arc::new(method_callback.handler);
        for method in method_callback.methods {
            top_branch.method_callbacks.insert(method, handler.clone());
        }
        //top_branch.method_callbacks.insert(method_callback.method, Arc::new(method_callback.handler));
        self
    }

    /// Adds a default method responder, in case no specific handler is found for the requested method.
    ///
    /// By default, unmatched methods reply with a `405 Method Not Allowed`, but this function allows override of such behaviour.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response, Method}};
    /// let branch: Branch<()> = Branch::new("/").with(Method::Get.to(|| async {
    ///     Response::ok().body("Supported!")
    /// })).unmatched_method_to(|| async {
    ///     Response::ok().body("Unsupported, please try with GET")
    /// });
    /// ```
    pub fn unmatched_method_to<F: Callback<A> + Send + Sync + 'static, A: Extractor<T>>(mut self, callback: F) -> Self {
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.default_method_callback = Some(Arc::new(Box::new(move |req: Request, additional: Arc<Additional<T>>|  {
            match <A as Extractor<T>>::extract(&req, additional) {
                Ok(args) => callback.invoke(args).boxed(),
                Err(_e) => {
                    #[cfg(feature = "full_log")]
                    {
                        log::error!("extractor error: {}", _e);
                        let response = _e.as_response();
                        (async {response}).boxed()
                    }
                    #[cfg(not(feature = "full_log"))]
                    {
                        (async {Response::bad_request()}).boxed()
                    }
                }
            }
        })));
        self
    }

    /// Adds a default callback, in case of no nested matching.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response}}; 
    /// // This branch will reply in any of `/hello`, `/hello/world`, `/hello/a/b` ,etc.
    /// let branch: Branch<()> = Branch::new("/hello").defaults_to(|| async {
    ///     Response::ok().body("Are you lost?")
    /// });
    /// ```
    pub fn defaults_to<F: Callback<A> + Send + Sync + 'static, A: Extractor<T>>(mut self, callback: F) -> Self {
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.default_callback = Some(Arc::new(Box::new(move |req: Request, additional: Arc<Additional<T>>|  {
            match <A as Extractor<T>>::extract(&req, additional) {
                Ok(args) => callback.invoke(args).boxed(),
                Err(_e) => {
                    #[cfg(feature = "full_log")]
                    {
                        log::error!("extractor error: {}", _e);
                        let response = _e.as_response();
                        (async {response}).boxed()
                    }
                    #[cfg(not(feature = "full_log"))]
                    {
                        (async {Response::bad_request()}).boxed()
                    }
                }
            }
        })));
        self
    }

    /// Allows static file serving.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response}}; 
    /// // This branch will reply with the default function to any
    /// // path that has no extension. If it has extension, static files
    /// // are served from ./static
    /// let branch: Branch<()> = Branch::new("/").defaults_to(|| async {
    ///     Response::ok().body("Is this an SPA?")
    /// }).files("./static");
    /// ```
    pub fn files<A: Into<PathBuf>>(mut self, files_location: A) -> Self {
        let fl = files_location.into();
        // For some odd reason, the compiler didn't guess this closure properly. So we help it :)
        let close: Box<dyn Fn(Request, Arc<Additional<T>>) -> Pin<Box<(dyn futures::Future<Output = Response> + Send + 'static)>> + Sync + Send> = Box::new(move |req: Request, _additional: Arc<Additional<T>>|  {
            let mut fl_clone = fl.clone();
            (async move {
                let trimmed_trail = req.url().path().trim_start_matches("/");
                let tokens = trimmed_trail.tokenize();
                let path: PathBuf = tokens.iter().skip(req.header.depth).collect();
                fl_clone.push(path);
                let extension = match fl_clone.extension().map(|e| e.to_str()).flatten() {
                    Some(e) => e,
                    None => return Response::internal_server_error()
                };
                match File::open(&fl_clone) {
                    Ok(mut f) =>  {
                        let mut content = Vec::new();
                        match f.read_to_end(&mut content) {
                            Ok(_) => (),
                            Err(_) => return Response::internal_server_error()
                        }
                        #[cfg(feature = "full_log")]
                        log::trace!("serving file {}", fl_clone.display());
                        Response::ok().body(content).header("Content-Type", crate::http::MIME_TYPES.get(extension).map(|v| *v).unwrap_or("application/octet-stream"))
                    },
                    Err(_) => {
                        #[cfg(feature = "full_log")]
                        log::debug!("file {} not found", fl_clone.display());
                        Response::not_found()
                    }
                }
            }).boxed()
        });
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.files_callback = Some(Arc::new(close));
        self
    }

    /// Helper for creating a file-loader default endpoint, for one specific file.
    ///
    /// This is useful for single page applications.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response}}; 
    /// // This is an SPA.
    /// let branch: Branch<()> = Branch::new("/")
    ///     .defaults_to_file("./static/index.html")
    ///     .files("./static");
    /// ```
    pub fn defaults_to_file<A: Into<PathBuf>>(mut self, file_location: A) -> Self {
        let fl = file_location.into();
        // For some odd reason, the compiler didn't guess this closure properly. So we help it :)
        let close: Box<dyn Fn(Request, Arc<Additional<T>>) -> Pin<Box<(dyn futures::Future<Output = Response> + Send + 'static)>> + Sync + Send> = Box::new(move |_req: Request, _additional: Arc<Additional<T>>|  {
            let fl_clone = fl.clone();
            (async move {
                let extension = match fl_clone.extension().map(|e| e.to_str()).flatten() {
                    Some(e) => e,
                    None => return Response::internal_server_error()
                };
                match File::open(&fl_clone) {
                    Ok(mut f) =>  {
                        let mut content = Vec::new();
                        match f.read_to_end(&mut content) {
                            Ok(_) => (),
                            Err(_) => return Response::internal_server_error()
                        }
                        Response::ok().body(content).header("Content-Type", crate::http::MIME_TYPES.get(extension).map(|s| *s).unwrap_or("application/octet-stream"))
                    },
                    Err(_) => Response::not_found()
                }
            }).boxed()
        });
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.default_callback = Some(Arc::new(close));
        self
    }

    /// Merges two paths, without taking control of the original path
    ///
    /// All priority is set to the caller
    fn merge_mut(&mut self, other: Branch<T>) {
        let Branch{
            exact_branches,
            pattern_branches,
            variable_branch,
            method_callbacks,
            default_method_callback,
            default_callback,
            files_callback,
            #[cfg(feature = "stream")]
            stream_handler,
            ..
        } = other;
        // If an exact match is found, we merge
        for (base, branch) in exact_branches.into_iter() {
            if let Some(eb) = self.exact_branches.get_mut(&base) {
                eb.merge_mut(branch);
            } else {
                self.exact_branches.insert(base, branch);
            }
        }

        // Priority to the lhs branch
        let mut additional_pattern_branches = Vec::new();
        for (rhs_pattern, rhs_branch) in pattern_branches.into_iter() {
            let mut remaining_rhs_branch = Some(rhs_branch);
            for (lhs_pattern, lhs_branch) in self.pattern_branches.iter_mut() {
                if lhs_pattern.as_str() == rhs_pattern.as_str() {
                    // Then we merge them
                    lhs_branch.merge_mut(remaining_rhs_branch.take().unwrap());
                    break;
                }
            }
            if let Some(rhs_branch) = remaining_rhs_branch {
                additional_pattern_branches.push((rhs_pattern, rhs_branch));
            }
        }
        self.pattern_branches.extend(additional_pattern_branches);

        // Priority to the other branch
        if self.variable_branch.is_none() {
            self.variable_branch = variable_branch;
        }

        //** Now the callbacks in this node **//

        // We add the method callbacks, priority to the other node
        for (method, callback) in method_callbacks.into_iter() {
            self.method_callbacks.entry(method).or_insert(callback);
        }

        // Priority for the lhs branch
        if self.default_method_callback.is_none() {
            self.default_method_callback = default_method_callback;
        }

        // Priority for the lhs branch
        if self.default_callback.is_none() {
            self.default_callback = default_callback;
        }

        // Priority for the lhs branch
        if self.files_callback.is_none() {
            self.files_callback = files_callback;
        }

        #[cfg(feature = "stream")]
        // Priority for the lhs branch
        if self.stream_handler.is_none() {
            self.stream_handler = stream_handler;
        }
    }

    /// Merges two branches from their bases, in case you find it useful
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Method, Response}};
    /// let branch_1: Branch<()> = Branch::new("/hello/world")
    ///     .with(Method::Get.to(|| async {Response::ok()}));
    /// let branch_2 = Branch::new("/hallo/welt")
    ///     .with(Method::Get.to(|| async {Response::unauthorized()}));
    /// // Merged branch will reply in `/hello/world` and in `/hallo/welt`
    /// let merged_branch = branch_1.merge(branch_2);
    /// ```
    ///
    /// Importance is held by the caller branch (`lhs`). That means that the following will hold true:
    ///
    /// * Method callbacks from `rhs` are only merged if not already present in `lhs`.
    /// * Exact matches from `rhs` will be merged if already found in `lhs`, else they get inserted.
    /// * Pattern matches from `rhs` will be marged if matched literally to another regex, else they will be inserted at the end of the evaluation queue.
    /// * Variable match from `rhs` is ignored if `lhs` already contains one.
    /// * Static file serving from `rhs` is ignored if `lhs` already contains one.
    pub fn merge(mut self, other: Branch<T>) -> Branch<T> {
        self.merge_mut(other);
        self
    }

    /// Nests one branch in the top node of the first one
    ///
    /// The "top node" is defined as the one following the path given to the branch constructor.
    ///
    /// ```rust
    /// # use cataclysm::Branch;
    /// let to_be_nested: Branch<()> = Branch::new("/world");
    /// // This one will reply in `/hello/world`
    /// let branch = Branch::new("/hello").nest(to_be_nested);
    /// ```
    pub fn nest(mut self, other: Branch<T>) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        // This unwrap looks risky, but I swear it is safe
        let top_branch = self.get_branch(source).unwrap();
        top_branch.merge_mut(other);
        self
    }

    /// Adds a processing layer to the callbacks contained in this branch
    ///
    /// A layer is what is commonly known as middleware. The passed layer methods act as a wrap to the core handling functions of this branch. It is important to note that layer functions have a very specific structure: each one receives a [`Request`](crate::http::Request) and a boxed [`Pipeline`](crate::Pipeline). The function must return a pinned boxed future. A Timing Layer/Middleware function is provided as an example.
    ///
    /// ```
    /// use cataclysm::{Branch, Additional, Pipeline, http::{Request, Response, Method}};
    /// use futures::future::FutureExt;
    /// use std::sync::Arc;
    /// 
    /// let branch = Branch::new("/hello")
    ///     .with(Method::Get.to(|| async {Response::ok().body("¡Hola!")}))
    ///     .layer(|req: Request, pipeline: Box<Pipeline<()>>, ad: Arc<Additional<()>>| async {
    ///         // Example of timing layer / middleware
    ///         let now = std::time::Instant::now();
    ///         // Execute the deeper layers of the pipeline, passing the request
    ///         let response = pipeline.execute(req, ad).await;
    ///         // Measure and print time
    ///         let elapsed = now.elapsed().as_nanos();
    ///         println!("Process time: {} ns", elapsed);
    ///         // We return the request for further possible processing.
    ///         response
    ///     }.boxed()
    /// );
    /// ```
    ///
    /// Calling the function multiple times will wrap the preceeding layer (or core handlers), like an onion 🧅.
    pub fn layer<F: 'static + Fn(Request, Box<Pipeline<T>>, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>(mut self, layer_fn: F) -> Self {
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.layers.push(Arc::new(Box::new(layer_fn)));
        self
    }

    /// Callback handler for direct stream manipulation
    /// 
    /// ```rust,no_run
    /// use cataclysm::{Server, Branch, Stream};
    /// use std::sync::Arc;
    /// 
    /// async fn deal_with_stream(stream: Stream) {
    ///     // do something with the stream...
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let branch: Branch<()> = Branch::new("/")
    ///         .nest(Branch::new("/ws").stream_handler(deal_with_stream))
    ///         .files("./static")
    ///         .defaults_to_file("./static/index.html");
    ///     let server = Server::builder(branch).build().unwrap();
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    #[cfg(feature = "stream")]
    pub fn stream_handler<F: StreamCallback<A> + Send + Sync + 'static, A: Extractor<T>>(mut self, handler: F) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.stream_handler = Some(Arc::new(Box::new(move |req: Request, additional: Arc<Additional<T>>, stream: Stream|  {
            match <A as Extractor<T>>::extract(&req, additional) {
                Ok(args) => handler.invoke(stream, args).boxed(),
                Err(_e) => {
                    #[cfg(feature = "full_log")]
                    {
                        log::error!("extractor error: {}", _e);
                        let response = _e.as_response();
                        // We use the stream to send the request
                        (async move {match stream.response(response).await {
                            Ok(_) => (),
                            Err(_e) => {
                                #[cfg(feature = "full_log")]
                                log::debug!("stream reply error: {}", _e);
                            }
                        };}).boxed()
                    }
                    #[cfg(not(feature = "full_log"))]
                    {
                        // We use the stream to send the request
                        (async move {match stream.response(Response::bad_request()).await {
                            Ok(_) => (),
                            Err(_e) => {
                                #[cfg(feature = "full_log")]
                                log::debug!("stream reply error: {}", _e);
                            }
                        };}).boxed()
                    }
                }
            }
        })));
        self
    }

    /// Turns the Branch into a PureBranch, basically getting rid of the "source" variable, and creating some callbacks.
    ///
    /// Internal use only. It helps because the tree structure won't change after this.
    pub(crate) fn purify(self) -> PureBranch<T> {
        PureBranch {
            exact_branches: self.exact_branches.into_iter().map(|(base, bb)| (base, bb.purify())).collect(),
            pattern_branches: self.pattern_branches.into_iter().map(|(base, bb)| (base, bb.purify())).collect(),
            variable_branch: self.variable_branch.map(|(var_id, bb)| (var_id, Box::new(bb.purify()))),
            method_callbacks: self.method_callbacks,
            default_method_callback: self.default_method_callback,
            default_callback: self.default_callback,
            files_callback: self.files_callback,
            layers: self.layers,
            #[cfg(feature = "stream")]
            stream_handler: self.stream_handler
        }
    }

    /// Gives back a node of the tree, if found.
    ///
    /// Used during branch construction only.
    fn get_branch<A: AsRef<str>>(&mut self, trail: A) -> Option<&mut Branch<T>> {
        // Tokenizamos la cadena
        let trimmed_trail = trail.as_ref().trim_start_matches("/");
        let (base, rest) = if let Some((base, rest)) = trimmed_trail.tokenize_once() {
            (base.to_string(), rest.to_string())
        } else {
            // Sólo hay un token
            if trimmed_trail.is_empty() {
                // Y es nulo
                return Some(self)
            } else {
                (trimmed_trail.to_string(), "".to_string())
            }
        };

        if let Some(branch) = self.exact_branches.get_mut(&base) {
            return branch.get_branch(rest)
        }
        for (pattern, branch) in self.pattern_branches.iter_mut() {
            if format!("{{regex:{}}}", pattern.as_str()) == base {
                return branch.get_branch(rest);
            }
        }
        if let Some((id, branch)) = &mut self.variable_branch {
            if format!("{{:{}}}", id) == base {
                return branch.get_branch(rest);
            }
        }
        None
    }

    /// Clasifies each token from the path in one of the three possibilities.
    fn clasify<A: AsRef<str>>(candidate: A) -> BranchKind {
        let default_re = Regex::new(r"^\{:.*\}$").unwrap();
        let regex_re = Regex::new(r"^\{regex:.*\}$").unwrap();
        if default_re.is_match(candidate.as_ref()) {
            BranchKind::Default
        } else if regex_re.is_match(candidate.as_ref()) {
            BranchKind::Pattern
        } else {
            BranchKind::Exact
        }
    }
}

/// Structure that holds information to process a callback properly
enum CallbackInformation<T> {
    ResponseHandler {
        #[cfg(feature = "full_log")]
        tracker: PipelineTrack,
        callback: Arc<CoreFn<T>>,
        layers: Vec<Arc<LayerFn<T>>>,
        variable_indicators: Vec<bool>
    },
    #[cfg(feature = "stream")]
    StreamHandler {
        #[cfg(feature = "full_log")]
        tracker: PipelineTrack,
        callback: Arc<HandlerFn<T>>,
        variable_indicators: Vec<bool>
    }
}

impl<T> CallbackInformation<T> {
    #[cfg(feature = "full_log")]
    fn tracker(&self) -> PipelineTrack {
        match self {
            CallbackInformation::ResponseHandler{tracker,..} => {
                tracker.clone()
            },
            #[cfg(feature = "stream")]
            CallbackInformation::StreamHandler{tracker, ..} => {
                tracker.clone()
            }
        }
    }

    fn update(&mut self, layers: Vec<Arc<LayerFn<T>>>, is_var: bool) {
        match self {
            CallbackInformation::ResponseHandler{layers: prev_layers, variable_indicators,..} => {
                // We append the possible layers from this level
                prev_layers.extend(layers);
                variable_indicators.push(is_var);
            },
            #[cfg(feature = "stream")]
            CallbackInformation::StreamHandler{variable_indicators, ..} => {
                variable_indicators.push(is_var);
            }
        }
    }

    #[cfg(feature = "full_log")]
    fn update_tracker<A: AsRef<str>>(&mut self, token: A) {
        match self {
            CallbackInformation::ResponseHandler{tracker ,..} => {
                tracker.preconcat(token);
            },
            #[cfg(feature = "stream")]
            CallbackInformation::StreamHandler{tracker, ..} => {
                tracker.preconcat(token);
            }
        }
    }
}

/// Structure for internal use only.
///
/// It is just a cleaner version of the Branch.
pub(crate) struct PureBranch<T> {
    exact_branches: HashMap<String, PureBranch<T>>,
    pattern_branches: Vec<(Regex, PureBranch<T>)>,
    variable_branch: Option<(String, Box<PureBranch<T>>)>,
    method_callbacks: HashMap<Method, Arc<CoreFn<T>>>,
    default_method_callback: Option<Arc<CoreFn<T>>>,
    default_callback: Option<Arc<CoreFn<T>>>,
    files_callback: Option<Arc<CoreFn<T>>>,
    layers: Vec<Arc<LayerFn<T>>>,
    #[cfg(feature = "stream")]
    stream_handler: Option<Arc<HandlerFn<T>>>
}

impl<T> PureBranch<T> {
    /// Creates the pipeline of futures to be processed by the server
    pub(crate) fn pipeline(&self, request: &mut Request) -> Option<PipelineInfo<T>> {
        // We get the core handler, and the possible layers
        if let Some(c_info) = self.callback_information(request.header.url.path(), &request.header.method) {
            #[cfg(feature = "full_log")]
            let pipeline_track = c_info.tracker();

            match c_info {
                CallbackInformation::ResponseHandler{callback, layers, variable_indicators, ..} => {
                    // We have to update the variable locations
                    request.header.depth = variable_indicators.len();

                    request.header.variable_indices = variable_indicators
                        .iter().rev().enumerate().filter(|(_idx, v)| **v)
                        .map(|(idx, _v)| idx).collect();

                    let mut pipeline_layer = Pipeline::Core(Arc::clone(&callback));
                    for function in &layers {
                        pipeline_layer = Pipeline::Layer(Arc::clone(function), Box::new(pipeline_layer));
                    }
                    // We return the nested pipeline
                    Some(PipelineInfo {
                        #[cfg(feature = "full_log")]
                        pipeline_track,
                        pipeline_kind: PipelineKind::NormalPipeline{pipeline: pipeline_layer}
                    })
                },
                #[cfg(feature = "stream")]
                CallbackInformation::StreamHandler{callback, variable_indicators, ..} => {
                    // We have to update the variable locations
                    request.header.depth = variable_indicators.len();

                    request.header.variable_indices = variable_indicators
                        .iter().rev().enumerate().filter(|(_idx, v)| **v)
                        .map(|(idx, _v)| idx).collect();
                    
                    Some(PipelineInfo{
                        #[cfg(feature = "full_log")]
                        pipeline_track,
                        pipeline_kind: PipelineKind::StreamPipeline{pipeline: callback}
                    })
                }
            }
        } else {
            None
        }
    }

    /// Gives back the supported methods on each path, in case the branch was found
    pub fn supported_methods<A: AsRef<str>>(&self, trail: A) -> Option<HashSet<Method>> {
        // Tokenizamos la cadena
        let trimmed_trail = trail.as_ref().trim_start_matches("/");
        
        let (base, rest) = if let Some((base, rest)) = trimmed_trail.tokenize_once() {
            (base.to_string(), rest.to_string())
        } else {
            // Only one token here
            if trimmed_trail.is_empty() {
                return if self.default_callback.is_some() || self.default_method_callback.is_some() {
                    Some(vec![Method::Get, Method::Post, Method::Put, Method::Head, Method::Delete, Method::Patch, Method::Options].into_iter().collect())
                } else {
                    let methods: HashSet<_> = self.method_callbacks.keys().map(|m| m.clone()).collect();
                    Some(methods)
                }
            } else {
                (trimmed_trail.to_string(), "".to_string())
            }
        };

        // First, exact matching through hash lookup
        let mut result = None;

        if let Some(branch) = self.exact_branches.get(&base) {
            result = branch.supported_methods(rest);
        } else {
            // Now, O(n) regex pattern matching
            for (pattern, branch) in self.pattern_branches.iter() {
                if pattern.is_match(&base) {
                    result = branch.supported_methods(&rest);
                    break;
                }
            }

            if result.is_none() {
                // Finally, if there is a variable, we reply (constant time)
                if let Some((_id, branch)) = &self.variable_branch {
                    result = branch.supported_methods(rest);
                }
            }
        }

        if result.is_none() {
            // We check if we are checking out a file, and there is a file callback
            if std::path::Path::new(trimmed_trail).extension().is_some() {
                if self.files_callback.is_some() {
                    result = Some(vec![Method::Get].into_iter().collect());
                }
            }
            
            if result.is_none() && self.default_callback.is_some() {
                result = Some(vec![Method::Get, Method::Post, Method::Put, Method::Head, Method::Delete, Method::Patch, Method::Options].into_iter().collect());
            }
        }

        result
    }

    /// Gives back the callback information
    ///
    /// For internal use only. This function shares code with the `supported_methods` function. Requires some way to abstract it.
    fn callback_information<A: AsRef<str>>(&self, trail: A, method: &Method) -> Option<CallbackInformation<T>> {
        // Tokenizamos la cadena, quitando la primer diagonal si es que existe
        let trimmed_trail = trail.as_ref().trim_start_matches("/");
        
        let (base, rest) = if let Some((base, rest)) = trimmed_trail.tokenize_once() {
            // Corta en la primer diagonal que encuentre, dejando lo demás en rest
            (base.to_string(), rest.to_string())
        } else {
            // Ya sólo queda un token aquí.
            if trimmed_trail.is_empty() {
                // Estamos en el endpoint final de la cadena
                return if let Some(mc) = self.method_callbacks.get(method) {
                    Some(CallbackInformation::ResponseHandler {
                        #[cfg(feature = "full_log")]
                        tracker: PipelineTrack::Exact("".to_string()),
                        callback: mc.clone(),
                        layers: self.layers.clone(),
                        variable_indicators: vec![]
                    })
                } else if let Some(dmc) = &self.default_method_callback {
                    Some(CallbackInformation::ResponseHandler {
                        #[cfg(feature = "full_log")]
                        tracker: PipelineTrack::UnmatchedMethod("".to_string()),
                        callback: dmc.clone(),
                        layers: self.layers.clone(),
                        variable_indicators: vec![]
                    })
                } else if let Some(dc) = &self.default_callback {
                    Some(CallbackInformation::ResponseHandler {
                        #[cfg(feature = "full_log")]
                        tracker: PipelineTrack::Default("".to_string()),
                        callback: dc.clone(),
                        layers: self.layers.clone(),
                        variable_indicators: vec![]
                    })
                } else {
                    #[cfg(feature = "stream")]
                    {
                        if let Some(sh) = &self.stream_handler {
                            Some(CallbackInformation::StreamHandler {
                                #[cfg(feature = "full_log")]
                                tracker: PipelineTrack::Stream("".to_string()),
                                callback: sh.clone(),
                                variable_indicators: vec![]
                            })
                        } else {
                            None
                        }
                    }
                    #[cfg(not(feature = "stream"))]
                    {
                        None
                    }
                };
            } else {
                (trimmed_trail.to_string(), "".to_string())
            }
        };

        // Si llegamos aquí, quiere decir que aún debemos hacer match de rama
        let mut result = None;
        // Indicator of a variable part of the route
        let mut is_var = true;

        if let Some(branch) = self.exact_branches.get(&base) {
            // Hubo un match exacto con rama exacta
            is_var = false;
            result = branch.callback_information(rest, method);
        } else {
            // Iteramos por todas las ramas que tienen regex, tiempo O(n)
            for (pattern, branch) in self.pattern_branches.iter() {
                if pattern.is_match(&base) {
                    result = branch.callback_information(&rest, method);
                    break;
                }
            }

            if result.is_none() {
                // Si hay rama con variable, aquí se llama de inmediato
                if let Some((_id, branch)) = &self.variable_branch {
                    result = branch.callback_information(rest, method);
                }
            }
        }

        match result.iter_mut().next() {
            Some(c_info) => {
                // Hubo una coincidencia, concatenamos capas si es que existen, y añadimos los indicadores de variables
                c_info.update(self.layers.clone(), is_var);

                #[cfg(feature = "full_log")]
                {
                    c_info.update_tracker(&base);
                }
            },
            None => {
                // No hubo coincidencia alguna. Podría ser un archivo y el endpoint de archivos estar habilitado
                if std::path::Path::new(trimmed_trail).extension().is_some() {
                    if let Some(fc) = &self.files_callback {
                        result = Some(CallbackInformation::ResponseHandler {
                            #[cfg(feature = "full_log")]
                            tracker: PipelineTrack::File("".to_string()),
                            callback: Arc::clone(fc),
                            layers: self.layers.clone(),
                            variable_indicators: vec![]
                        });
                    }
                }

                // Si llegamos aquí, ya sólo queda probar el callback por defecto.
                if result.is_none() {
                    if let Some(dc) = &self.default_callback {
                        result = Some(CallbackInformation::ResponseHandler {
                            #[cfg(feature = "full_log")]
                            tracker: PipelineTrack::Default("".to_string()),
                            callback: Arc::clone(dc),
                            layers: self.layers.clone(),
                            variable_indicators: vec![]
                        });
                    }
                }
            }
        }

        result
    }
}

// Helper trait to split the path, even with regex components that contain a slash
pub(crate) trait Tokenizable {
    /// A replacement for split("/") that detects regex
    fn tokenize(&self) -> Vec<&str>;
    /// Basically a replacement for split_once("/") that detects regex
    fn tokenize_once(&self) -> Option<(&str, &str)>;
}

impl<T: AsRef<str>> Tokenizable for T {
    fn tokenize(&self) -> Vec<&str> {
        let r = self.as_ref();
        let (a, b) = (r.char_indices(), r.char_indices().skip(1));

        let mut tokens = vec![];
        let mut prev_pos = 0;
        for val in a.zip(b) {
            if val.1.1 == '/' && val.0.1 != '\\' {
                // We cut!
                tokens.push(r.get(prev_pos..val.1.0).unwrap());
                prev_pos = val.1.0 + 1;
            }
        }
        if prev_pos != r.len() {
            tokens.push(r.get(prev_pos..r.len()).unwrap());
        }
        tokens
    }

    fn tokenize_once(&self) -> Option<(&str, &str)> {
        let r = self.as_ref();
        let (a, b) = (r.char_indices(), r.char_indices().skip(1));
        for val in a.zip(b) {
            if val.1.1 == '/' && val.0.1 != '\\' {
                // We cut!
                return Some((r.get(0..val.1.0).unwrap(), r.get((val.1.0+1)..r.len()).unwrap()));
            }
        }
        None
    }
}