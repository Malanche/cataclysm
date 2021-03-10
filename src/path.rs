use crate::http::{Response, Method, MethodHandler};
use std::future::Future;
use std::collections::HashMap;

pub struct Path {
    original: String
}

impl Path {
    pub fn new<T: Into<String>>(original: T) -> Path {
        Path {
            original: original.into()
        }
    }

    pub fn such_that<F: Fn() -> T, T: Future<Output = Response>>(self, method_handler: MethodHandler<F, T>) -> Self {
        //self.handler = method_handler;
        self
    }

    pub(crate) fn get_root(&self) -> String {
        self.original.clone()
    }
}

//let path = Path::new("/hello").replies(Method::Get.with(my_function));