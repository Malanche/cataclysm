use std::future::Future;

use crate::{Processor, http::{Response}};

/// Available methods for HTTP Requests
#[derive(PartialEq, Hash)]
pub enum Method {
    /// Get method
    Get,
    /// Post method
    Post,
    /// Put method
    Put,
    /// Head method
    Head,
    /// Delete method
    Delete,
    /// Patch method
    Patch,
    /// Options method
    Options
}

impl Eq for Method{}

impl Method {
    /// Turns the Method into a MethodHandler, which is a short for a tuple Method - Handler
    pub fn to<F: Processor>(self, handler: F) -> MethodHandler<F> {
        MethodHandler{
            method: self,
            handler
        }
    }

    pub fn from_str<T: AsRef<str>>(source: T) -> Option<Method> {
        match source.as_ref() {
            "GET" | "get" => Some(Method::Get),
            "POST" | "post" => Some(Method::Post),
            "PUT" | "put" => Some(Method::Put),
            "HEAD" | "head" => Some(Method::Head),
            "DELETE" | "delete" => Some(Method::Delete),
            "PATCH" | "patch" => Some(Method::Patch),
            "OPTIONS" | "options" => Some(Method::Options),
            _ => None
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Head => "HEAD",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Options => "OPTIONS"
        }
    }
}

pub struct MethodHandler<F: Processor> {
    pub(crate) method: Method,
    pub(crate) handler: F
}