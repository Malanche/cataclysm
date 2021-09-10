use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use crate::{Callback, Extractor, http::{Response, Request}};

/// Available methods for HTTP Requests
#[derive(Clone, PartialEq, Hash)]
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
    pub fn to<F: Callback<A> + Send + Sync + 'static, A: Extractor>(self, handler: F) -> MethodHandler {
        MethodHandler{
            method: self,
            handler: Box::new(move |req: Request|  {
                //let args = <A as Extractor>::extract(&req);
                //handler.invoke(args).boxed()
                match <A as Extractor>::extract(&req) {
                    Ok(args) => handler.invoke(args).boxed(),
                    Err(e) => {
                        println!("{}", e);
                        (async {Response::bad_request()}).boxed()
                    }
                }
            })
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

pub struct MethodHandler {
    pub(crate) method: Method,
    pub(crate) handler: Box<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>
}