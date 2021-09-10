use std::collections::HashMap;
use regex::Regex;
use futures::future::FutureExt;
use crate::{
    CoreFn, LayerFn, Extractor, Callback, Pipeline,
    http::{Method, Request, Response, MethodHandler}
};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

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
/// let branch = Branch::new("/hello/world");
/// ```
///
/// Pattern matching is a bit more complex
///
/// ```rust
/// # use cataclysm::Branch;
/// // matches any route that starts with `/hello/` and then words of 3 or 4 letters, no numbers
/// let branch = Branch::new("/hello/{regex:^[A-Za-z\\d]{3,4}$}");
/// ```
///
/// Last but not least, we have variable detection, with no regex
///
/// ```rust
/// # use cataclysm::Branch;
/// // matches any route that contains "/hello/{:variable}"
/// let branch = Branch::new("/hello/{:variable}");
/// ```
pub struct Branch {
    /// Exact match branches
    exact_branches: HashMap<String, Branch>,
    /// Regex match branches
    pattern_branches: Vec<(Regex, Branch)>,
    /// Variable branch, only one per branch
    variable_branch: Option<(String, Box<Branch>)>,
    /// Original source that created the branch, to point to the top node
    source: String,
    /// Method Callbacks
    method_callbacks: HashMap<Method, Arc<CoreFn>>,
    /// Default method callback
    default_method_callback: Option<Arc<CoreFn>>,
    /// Default callback for this node, and all the non-matching children
    default_callback: Option<Arc<CoreFn>>,
    /// Layer functions on this branch
    layers: Vec<Arc<LayerFn>>
}

impl std::fmt::Display for Branch {
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
            content += &format!("\n--> regex: {}", pattern.as_str());
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

impl Branch {
    /// Creates a new branch
    ///
    /// ```rust
    /// # use cataclysm::Branch;
    /// let branch = Branch::new("/hello/world");
    /// ```
    pub fn new<A: AsRef<str>>(trail: A) -> Branch {
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

        match Branch::clasify(&base) {
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
    /// let branch = Branch::new("/scope").with(Method::Get.to(index));
    /// ```
    pub fn with(mut self, method_callback: MethodHandler) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        let top_branch = self.get_branch(source).unwrap();
        top_branch.method_callbacks.insert(method_callback.method, Arc::new(method_callback.handler));
        self
    }

    /// Merges two paths, without taking control of the original path
    fn merge_mut(&mut self, other: Branch) {
        let Branch{
            exact_branches,
            pattern_branches,
            variable_branch,
            method_callbacks,
            default_method_callback,
            default_callback, ..
        } = other;
        for (base, branch) in exact_branches.into_iter() {
            if let Some(eb) = self.exact_branches.get_mut(&base) {
                eb.merge_mut(branch);
            } else {
                self.exact_branches.insert(base, branch);
            }
        }
        // Priority to the self branch
        for (pattern, branch) in pattern_branches.into_iter() {
            self.pattern_branches.push((pattern, branch));
        }
        // Priority to the other branch
        if variable_branch.is_some() {
            self.variable_branch = variable_branch;
        }

        // We add the method callbacks, priority to the other node
        for (method, callback) in method_callbacks.into_iter() {
            self.method_callbacks.insert(method, callback);
        }

        // Priority for the other branch
        if default_method_callback.is_some() {
            self.default_method_callback = default_method_callback;
        }

        // Priority for the other branch
        if default_callback.is_some() {
            self.default_callback = default_callback;
        }
    }

    /// Merges two branches from their bases, in case you find it useful
    ///
    /// ```rust
    /// # use cataclysm::{Branch};
    /// let branch_1 = Branch::new("/hello/world");
    /// let branch_2 = Branch::new("/hallo/welt");
    /// // Replies to both branches, in theory
    /// let merged_branch = branch_1.merge(branch_2);
    /// ```
    ///
    /// Please note that the caller has precedence over the callee, so in case of layers, the layers of the left hand side will have precedence, as well as the regex matches and the variable callback.
    pub fn merge(mut self, other: Branch) -> Branch {
        self.merge_mut(other);
        self
    }

    /// Nests one branch in the top node of the first one
    ///
    /// The "top node" is defined as the one following the path given to the branch constructor.
    ///
    /// ```rust
    /// # use cataclysm::Branch;
    /// let to_be_nested = Branch::new("/world");
    /// // This one will reply in `/hello/world`
    /// let branch = Branch::new("/hello").nest(to_be_nested);
    /// ```
    pub fn nest(mut self, other: Branch) -> Self {
        // We get the top node from the current branch
        let source = self.source.clone();
        // This unwrap looks risky, but I swear it is safe
        let top_branch = self.get_branch(source).unwrap();
        top_branch.merge_mut(other);
        self
    }

    /// Adds a default method responder, in case no specific handler is found for the requested method.
    ///
    /// By default, unmatched methods reply with a `405 Method Not Allowed`, but this function allows override of such behaviour.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response, Method}};
    /// let branch = Branch::new("/").with(Method::Get.to(|| async {
    ///     Response::ok().body("Supported!")
    /// })).unmatched_method_to(|| async {
    ///     Response::ok().body("Unsupported, please try with GET")
    /// });
    /// ```
    pub fn unmatched_method_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, callback: F) -> Self {
        self.default_method_callback = Some(Arc::new(Box::new(move |req: Request|  {
            match <A as Extractor>::extract(&req) {
                Ok(args) => callback.invoke(args).boxed(),
                Err(e) => {
                    log::trace!("{}", e);
                    (async {Response::bad_request()}).boxed()
                }
            }
            //callback.invoke(args).boxed()
        })));
        self
    }

    /// Adds a default callback, in case of no nested matching.
    ///
    /// ```rust
    /// # use cataclysm::{Branch, http::{Response}}; 
    /// // This branch will reply in any of `/hello`, `/hello/world`, etc.
    /// let branch = Branch::new("/hello").defaults_to(|| async {
    ///     Response::ok().body("Are you lost?")
    /// });
    /// ```
    pub fn defaults_to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(mut self, callback: F) -> Self {
        self.default_callback = Some(Arc::new(Box::new(move |req: Request|  {
            //let args = <A as Extractor>::extract(&req);
            //callback.invoke(args).boxed()
            match <A as Extractor>::extract(&req) {
                Ok(args) => callback.invoke(args).boxed(),
                Err(e) => {
                    log::trace!("{}", e);
                    (async {Response::bad_request()}).boxed()
                }
            }
        })));
        self
    }

    /// Adds a processing layer to the callbacks contained in this branch
    ///
    /// A layer is what is commonly known as middleware. The passed layer methods act as a wrap to the core handling functions of this branch. It is important to note that layer functions have a very specific structure: each one receives a [`Request`](crate::http::Request) and a boxed [`Pipeline`](crate::Pipeline). The function must return a pinned boxed future. A Timing Layer/Middleware function is provided as an example.
    ///
    /// ```
    /// use cataclysm::{Branch, Pipeline, http::{Request, Response, Method}};
    /// use futures::future::FutureExt;
    /// 
    /// let branch = Branch::new("/hello")
    ///     .with(Method::Get.to(|| async {Response::ok().body("¡Hola!")}))
    ///     .layer(|req: Request, pipeline: Box<Pipeline>| async {
    ///         // Example of timing layer / middleware
    ///         let now = std::time::Instant::now();
    ///         // Execute the deeper layers of the pipeline, passing the request
    ///         let response = pipeline.execute(req).await;
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
    pub fn layer<F: Fn(Request, Box<Pipeline>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static>(mut self, layer_fn: F) -> Self {
        self.layers.push(Arc::new(Box::new(layer_fn)));
        self
    }

    /// Turns the Branch into a PureBranch, basically getting rid of the "source" variable.
    ///
    /// Internal use only.
    pub(crate) fn purify(self) -> PureBranch {
        PureBranch {
            exact_branches: self.exact_branches.into_iter().map(|(base, bb)| (base, bb.purify())).collect(),
            pattern_branches: self.pattern_branches.into_iter().map(|(base, bb)| (base, bb.purify())).collect(),
            variable_branch: self.variable_branch.map(|(var_id, bb)| (var_id, Box::new(bb.purify()))),
            method_callbacks: self.method_callbacks,
            default_method_callback: self.default_method_callback,
            default_callback: self.default_callback,
            layers: self.layers
        }
    }

    /// Gives back a node of the tree, if found.
    ///
    /// Used during branch construction only.
    fn get_branch<A: AsRef<str>>(&mut self, trail: A) -> Option<&mut Branch> {
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
struct CallbackInformation {
    callback: Arc<CoreFn>,
    layers: Vec<Arc<LayerFn>>,
    variable_indicators: Vec<bool>
}

/// Structure for internal use only.
///
/// It is just a cleaner version of the Branch.
pub(crate) struct PureBranch {
    exact_branches: HashMap<String, PureBranch>,
    pattern_branches: Vec<(Regex, PureBranch)>,
    variable_branch: Option<(String, Box<PureBranch>)>,
    method_callbacks: HashMap<Method, Arc<CoreFn>>,
    default_method_callback: Option<Arc<CoreFn>>,
    default_callback: Option<Arc<CoreFn>>,
    layers: Vec<Arc<LayerFn>>
}

impl PureBranch {
    /// Creates the pipeline of futures to be processed by the server
    pub(crate) fn pipeline(&self, request: &mut Request) -> Option<Pipeline> {
        // We get the core handler, and the possible layers
        if let Some(c_info) = self.callback_information(&request.path, &request.method) {
            // We have to update the variable locations
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
    fn callback_information<A: AsRef<str>>(&self, trail: A, method: &Method) -> Option<CallbackInformation> {
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
                if let Some(dc) = &self.default_callback {
                    result = Some(CallbackInformation {
                        callback: Arc::clone(dc),
                        layers: self.layers.clone(),
                        variable_indicators: vec![]
                    });
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