use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::{Callback, additional::Additional, Extractor, http::{Response, Request}};

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
    pub fn to<T: Sync, F: Callback<A> + Send + Sync + 'static, A: Extractor<T>>(self, handler: F) -> MethodHandler<T> {
        MethodHandler{
            method: self,
            handler: Box::new(move |req: Request, additional: Arc<Additional<T>>|  {
                match <A as Extractor<T>>::extract(&req, additional) {
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

pub struct MethodHandler<T = ()> {
    pub(crate) method: Method,
    pub(crate) handler: Box<dyn Fn(Request, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>
}