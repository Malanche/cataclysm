use std::collections::HashMap;
use regex::Regex;
use futures::future::FutureExt;
use crate::{
    additional::Additional,
    CoreFn, LayerFn, Extractor, Callback, Pipeline,
    http::{Method, Request, Response, MethodHandler}
};
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
/// Branches are cataclysm's main building block. It is a really simple pattern matching system, with the following priorities. They are named branches to avoid conflict with the [Branch](crate::Branch) extractor.
///
/// 1. Exact matching
/// 2. Pattern matching
/// 3. Default branches (a.k.a, variable handling in branches)
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
    layers: Vec<Arc<LayerFn<T>>>
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
    /// // Branch that will reply go a get method in `/scope`
    /// let branch: Branch<()> = Branch::new("/scope").with(Method::Get.to(index));
    /// ```
    pub fn with(mut self, method_callback: MethodHandler<T>) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.method_callbacks.insert(method_callback.method, Arc::new(method_callback.handler));
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
                Err(e) => {
                    log::trace!("{}", e);
                    (async {Response::bad_request()}).boxed()
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
                Err(e) => {
                    log::trace!("{}", e);
                    (async {Response::bad_request()}).boxed()
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
                let trimmed_trail = req.path.trim_start_matches("/");
                let tokens = trimmed_trail.tokenize();
                let path: PathBuf = tokens.iter().skip(req.depth).collect();
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
                        Response::ok().body(content).header("Content-Type", crate::http::MIME_TYPES.get(extension).map(|v| *v).unwrap_or("application/octet-stream"))
                    },
                    Err(_) => Response::not_found()
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
    pub fn layer<F: Fn(Request, Box<Pipeline<T>>, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static>(mut self, layer_fn: F) -> Self {
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.layers.push(Arc::new(Box::new(layer_fn)));
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
            layers: self.layers
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
struct CallbackInformation<T> {
    callback: Arc<CoreFn<T>>,
    layers: Vec<Arc<LayerFn<T>>>,
    variable_indicators: Vec<bool>
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
    layers: Vec<Arc<LayerFn<T>>>
}

impl<T> PureBranch<T> {
    /// Creates the pipeline of futures to be processed by the server
    pub(crate) fn pipeline(&self, request: &mut Request) -> Option<Pipeline<T>> {
        // We get the core handler, and the possible layers
        if let Some(c_info) = self.callback_information(&request.path, &request.method) {
            // We have to update the variable locations
            request.depth = c_info.variable_indicators.len();

            request.variable_indices = c_info.variable_indicators
                .iter().rev().enumerate().filter(|(_idx, v)| **v)
                .map(|(idx, _v)| idx).collect();

            let mut pipeline_layer = Pipeline::Core(Arc::clone(&c_info.callback));
            for function in &c_info.layers {
                pipeline_layer = Pipeline::Layer(Arc::clone(function), Box::new(pipeline_layer));
            }
            // We return the nested pipeline
            Some(pipeline_layer)
        } else {
            None
        }
    }

    /// Gives back the callback information
    ///
    /// For internal use only
    fn callback_information<A: AsRef<str>>(&self, trail: A, method: &Method) -> Option<CallbackInformation<T>> {
        // Tokenizamos la cadena
        let trimmed_trail = trail.as_ref().trim_start_matches("/");
        
        let (base, rest) = if let Some((base, rest)) = trimmed_trail.tokenize_once() {
            (base.to_string(), rest.to_string())
        } else {
            // Only one token here
            if trimmed_trail.is_empty() {
                return if let Some(mc) = self.method_callbacks.get(method) {
                    Some(mc.clone())
                } else if let Some(dmc) = &self.default_method_callback {
                    Some(dmc.clone())
                } else if let Some(dc) = &self.default_callback {
                    Some(dc.clone())
                } else {
                    None
                }.map(|callback| {
                    CallbackInformation {
                        callback,
                        layers: self.layers.clone(),
                        variable_indicators: vec![]
                    }
                });
            } else {
                (trimmed_trail.to_string(), "".to_string())
            }
        };

        // First, exact matching through hash lookup
        let mut result = None;
        // Indicator of a variable part of the route
        let mut is_var = true;

        if let Some(branch) = self.exact_branches.get(&base) {
            is_var = false;
            result = branch.callback_information(rest, method);
        } else {
            // Now, O(n) regex pattern matching
            for (pattern, branch) in self.pattern_branches.iter() {
                if pattern.is_match(&base) {
                    result = branch.callback_information(&rest, method);
                    break;
                }
            }

            if result.is_none() {
                // Finally, if there is a variable, we reply (constant time)
                if let Some((_id, branch)) = &self.variable_branch {
                    result = branch.callback_information(rest, method);
                }
            }
        }

        match result.iter_mut().next() {
            Some(c_info) => {
                // We append the possible layers from this level
                c_info.layers.extend(self.layers.clone());
                c_info.variable_indicators.push(is_var);
            },
            None => {
                // Now, there was not match at all. First, we verify if the path is a file
                if std::path::Path::new(trimmed_trail).extension().is_some() {
                    if let Some(fc) = &self.files_callback {
                        result = Some(CallbackInformation {
                            callback: Arc::clone(fc),
                            layers: self.layers.clone(),
                            variable_indicators: vec![]
                        });
                    }
                }

                // Last, if there is a default callback, we call it.
                if result.is_none() {
                    if let Some(dc) = &self.default_callback {
                        result = Some(CallbackInformation {
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