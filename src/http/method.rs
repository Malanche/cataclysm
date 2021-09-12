use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::{Callback, additional::Additional, Extractor, http::{Response, Request}};
use std::collections::HashSet;

/// Available methods for HTTP Requests
#[derive(Clone, PartialEq, Hash, Debug)]
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

/// Holds multiple methods to make callback management easier
#[derive(Debug)]
pub struct MultipleMethod(HashSet<Method>);

impl MultipleMethod {
    /// Turns the Method into a MethodHandler, which is a short for a tuple Method - Handler
    ///
    /// ```rust
    /// # use cataclysm::http::Method;
    /// let mul = Method::Put.and(Method::Post).and(Method::Patch);
    /// ```
    pub fn to<T: Sync, F: Callback<A> + Send + Sync + 'static, A: Extractor<T>>(self, handler: F) -> MethodHandler<T> {
        MethodHandler{
            methods: self.0,
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

    /// Adds another method
    ///
    /// ```rust
    /// # use cataclysm::http::Method;
    /// // This first and belongs to the `Method` struct
    /// let mul = Method::Put.and(Method::Post);
    /// // This one to the `MultipleMethod` struct
    /// let more_mul = mul.and(Method::Patch);
    /// ```
    pub fn and(mut self, rhs: Method) -> MultipleMethod {
        self.0.insert(rhs);
        self
    }
}

impl Method {
    /// Turns the Method into a MethodHandler, which is a short for a tuple Method - Handler
    pub fn to<T: Sync, F: Callback<A> + Send + Sync + 'static, A: Extractor<T>>(self, handler: F) -> MethodHandler<T> {
        MethodHandler{
            methods: vec![self].into_iter().collect(),
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

    /// Casts a method from a string.
    ///
    /// ```rust
    /// # use cataclysm::http::Method;
    /// assert_eq!(Some(Method::Get), Method::from_str("GET"));
    /// ```
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

    /// Retrieves the `str` representation of a method (all caps).
    ///
    /// ```rust
    /// # use cataclysm::http::Method;
    /// assert_eq!("GET", Method::Get.to_str());
    /// ```
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

    /// Makes a [MultipleMethod](crate::http::MultipleMethod)
    ///
    /// In case you want a function to reply to more than one method, you can put as many as you want with the `and` method.
    /// ```rust
    /// # use cataclysm::http::Method;
    /// let mul = Method::Get.and(Method::Post);
    /// ```
    pub fn and(self, rhs: Method) -> MultipleMethod {
        MultipleMethod(vec![self, rhs].into_iter().collect())
    }
}

/// Contains a group of methods, and a handler function.
pub struct MethodHandler<T = ()> {
    pub(crate) methods: HashSet<Method>,
    pub(crate) handler: Box<dyn Fn(Request, Arc<Additional<T>>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>
}