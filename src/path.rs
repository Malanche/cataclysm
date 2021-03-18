use crate::{Processor, http::{Response, MethodHandler}};
use std::future::Future;
use std::collections::HashMap;

/// Main building block for cataclysm 
pub struct Path {
    /// Tokenized path. An empty vec means it replies to the root
    pub(crate) tokenized_path: Vec<String>,
    /// Handler associated to the get method
    pub(crate) get_handler: Option<Box<dyn Processor + Send + Sync>>,
    /// Inner branches of the path
    pub(crate) branches: Vec<Path>
}

impl Path {
    pub fn new<T: Into<String>>(path_string: T) -> Path {
        let mut tokenized_path: Vec<_> = path_string.into().split("/").map(|v| v.to_string()).collect();
        tokenized_path.retain(|v| v.len() != 0);
        Path {
            tokenized_path: tokenized_path,
            get_handler: None,
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
    pub fn with<F: 'static + Processor + Sync + Send>(mut self, method_handler: MethodHandler<F>) -> Self {
        self.get_handler = Some(Box::new(method_handler.handler));
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
    pub fn nested(mut self, mut path: Path) -> Self {
        self.branches.push(path);
        self
    }
}

//let path = Path::new("/hello").replies(Method::Get.with(my_function));