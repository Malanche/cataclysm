use crate::{Processor, http::{Response, MethodHandler}};
use std::future::Future;
use std::collections::HashMap;

/// Main structure regarding 
pub struct Path {
    original: String,
    pub(crate) get_handler: Option<Box<dyn Processor + Send + Sync>>,
    pub(crate) branches: HashMap<String, Path>
}

async fn index() -> Response {
    Response::new().body(b"<div>Hello!!!</div>")
}

impl Path {
    pub fn new<T: Into<String>>(original: T) -> Path {
        Path {
            original: original.into(),
            get_handler: None,
            branches: HashMap::new()
        }
    }

    /// Adds a callback to a method or a group of methods
    pub fn with<F: 'static + Fn() -> T + Sync + Send, T: Future<Output = Response> + Send + Sync>(mut self, method_handler: MethodHandler<F, T>) -> Self {
        self.get_handler = Some(Box::new(method_handler.handler));
        self
    }

    /// Adds a nested path to the actual path
    pub fn nested(mut self, mut path: Path) -> Self {
        // First, we have to strip the path, using an iterator
        let mut elements = path.original.split("/");
        let first = elements.next().unwrap();
        if first.len() == 0 {
            // We use the next element, as the path started with /
            let first = elements.next().unwrap().to_string();
            path.original = elements.collect::<Vec<_>>().join("/");
            self.branches.insert(first, path);
        } else {
            self.branches.insert(first.to_string(), path);
        }
        self
    }

    pub(crate) fn get_root(&self) -> String {
        self.original.clone()
    }
}

//let path = Path::new("/hello").replies(Method::Get.with(my_function));