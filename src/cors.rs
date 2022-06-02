use std::collections::HashSet;
use crate::{Error, http::{Request, Response, Method}};
use url::{Url, Origin};

/// Cors builder structure
pub struct CorsBuilder {
    origins: HashSet<String>,
    max_age: Option<usize>,
    methods: Option<HashSet<Method>>
}

impl CorsBuilder {
    /// Creates a [CorsBuilder](CorsBuilder) instance
    pub fn new() -> CorsBuilder {
        CorsBuilder {
            origins: HashSet::new(),
            max_age: None,
            methods: None
        }
    }

    /// Adds an allowed origin
    pub fn origin<A: Into<String>>(mut self, origin: A) -> Self {
        self.origins.insert(origin.into());
        self
    }

    /// Adds an allowed origin
    pub fn max_age(mut self, seconds: usize) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Adds an allowed method to be used
    ///
    /// By default, if no method is provided, cataclysm will use the callbacks and their methods to construct a response
    pub fn allowed_method(mut self, method: Method) -> Self {
        self.methods.get_or_insert_with(|| HashSet::new()).insert(method);
        self
    }

    /// Builds de cors object
    pub fn build(self) -> Result<Cors, Error> {
        Ok(Cors {
            origins: self.origins.into_iter().map(|origin| {
                Url::parse(&origin)
            }).collect::<Result<Vec<_>,_>>().map_err(Error::Url)?
            .into_iter().map(|url| url.origin()).collect(),
            max_age: self.max_age,
            methods: self.methods
        })
    }
}

/// Inner cors structure
///
/// This structure cannot be created directly. Use the [CorsBuilder](CorsBuilder) structure.
pub struct Cors {
    origins: HashSet<Origin>,
    max_age: Option<usize>,
    methods: Option<HashSet<Method>>
}

impl Cors {
    pub(crate) fn apply(&self, request: &Request, response: &mut Response) {
        if let Some(origin) = request.headers.get("Origin") {
            match Url::parse(&origin) {
                Ok(url) => {
                    if let Some(found_origin) = self.origins.get(&url.origin()) {
                        // It should reply
                        response.headers.insert(
                            "Access-Control-Allow-Origin".to_string(),
                            found_origin.ascii_serialization()
                        );

                        if let Some(max_age) = self.max_age {
                            response.headers.insert(
                                "Access-Control-Max-Age".to_string(),
                                format!("{}", max_age)
                            );
                        }
                    }
                },
                Err(_e) => {
                    #[cfg(feature = "full_log")]
                    log::debug!("{}", _e);
                }
            }
        }
    }

    /// Computed the preflight response
    pub(crate) fn preflight(&self, request: &Request, methods: HashSet<Method>) -> Response {
        if let Some(origin) = request.headers.get("Origin") {
            match Url::parse(&origin) {
                Ok(url) => {
                    if let Some(found_origin) = self.origins.get(&url.origin()) {
                        let mut response = Response::no_content();
                        // It should reply
                        response.headers.insert(
                            "Access-Control-Allow-Origin".to_string(),
                            found_origin.ascii_serialization()
                        );

                        let methods = if let Some(override_methods) = &self.methods {
                            override_methods.iter()
                        } else {
                            methods.iter()
                        }.map(|m| m.to_str()).collect::<Vec<_>>().join(", ");

                        response.headers.insert(
                            "Access-Control-Allow-Methods".to_string(),
                            methods
                        );

                        if let Some(max_age) = self.max_age {
                            response.headers.insert(
                                "Access-Control-Max-Age".to_string(),
                                format!("{}", max_age)
                            );
                        }

                        return response;
                    }
                },
                Err(_e) => {
                    #[cfg(feature = "full_log")]
                    log::debug!("{}", _e);
                }
            }
        }

        Response::unauthorized()
    }
}