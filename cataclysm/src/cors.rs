use std::collections::HashSet;
use crate::{Error, http::{Request, Response, Method}};
use url::{Url, Origin};

/// Handy enum to deal with the * wildcard
enum CorsOriginBuilder {
    None,
    All,
    List(HashSet<String>)
}

impl CorsOriginBuilder {
    fn build(self) -> Result<CorsOrigin, Error> {
        match self {
            CorsOriginBuilder::None => Ok(CorsOrigin::None),
            CorsOriginBuilder::All => Ok(CorsOrigin::All),
            CorsOriginBuilder::List(origins) => {
                Ok(CorsOrigin::List(origins.into_iter().map(|origin| {
                    Url::parse(&origin)
                }).collect::<Result<Vec<_>,_>>().map_err(Error::Url)?
                .into_iter().map(|url| url.origin()).collect()))
            }
        }
    }
}

enum CorsOrigin {
    None,
    All,
    List(HashSet<Origin>)
}

/// Cors builder structure
pub struct CorsBuilder {
    origins: CorsOriginBuilder,
    max_age: Option<usize>,
    methods: Option<HashSet<Method>>,
    headers: Option<HashSet<String>>
}

impl CorsBuilder {
    /// Creates a [CorsBuilder](CorsBuilder) instance
    pub fn new() -> CorsBuilder {
        CorsBuilder {
            origins: CorsOriginBuilder::None,
            max_age: None,
            methods: None,
            headers: None
        }
    }

    /// Adds an allowed origin
    ///
    /// By default, if this method is never called, not a single response will be different from "forbidden"
    pub fn origin<A: Into<String>>(mut self, origin: A) -> Self {
        let origin: String = origin.into();
        match &mut self.origins {
            CorsOriginBuilder::All => (),
            CorsOriginBuilder::List(origins) => {
                if origin == "*" {
                    self.origins = CorsOriginBuilder::All;
                } else {
                    origins.insert(origin);
                }
            },
            CorsOriginBuilder::None => {
                if origin == "*" {
                    self.origins = CorsOriginBuilder::All;
                } else {
                    self.origins = CorsOriginBuilder::List([origin].into_iter().collect());
                }
            }
        }
        self
    }

    /// Adds an allowed origin
    pub fn max_age(mut self, seconds: usize) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Adds an allowed method to be used for preflight requests
    ///
    /// By default, if no methods are provided, cataclysm will use the callbacks and their methods to construct a response
    pub fn allowed_method(mut self, method: Method) -> Self {
        self.methods.get_or_insert_with(|| HashSet::new()).insert(method);
        self
    }

    /// Adds an allowed header to be used
    ///
    /// By default, if no header is provided, cataclysm will mirror the headers listed in the `Access-Control-Request-Headers` field. Please use with caution.
    pub fn allowed_header<A: Into<String>>(mut self, header: A) -> Self {
        self.headers.get_or_insert_with(|| HashSet::new()).insert(header.into());
        self
    }

    /// Builds de cors object
    pub fn build(self) -> Result<Cors, Error> {
        Ok(Cors {
            origins: self.origins.build()?,
            max_age: self.max_age,
            methods: self.methods,
            headers: self.headers
        })
    }
}

/// Inner cors structure
///
/// This structure cannot be created directly. Use the [CorsBuilder](CorsBuilder) structure.
pub struct Cors {
    origins: CorsOrigin,
    max_age: Option<usize>,
    methods: Option<HashSet<Method>>,
    headers: Option<HashSet<String>>
}

impl Cors {
    pub(crate) fn apply(&self, request: &Request, response: &mut Response) {
        let origin_source = request.headers.get("Origin").map(|o| o.get(0)).flatten().or_else(||
            request.headers.get("origin").map(|o| o.get(0)).flatten()
        );
        let acao = match &self.origins {
            CorsOrigin::None => None,
            CorsOrigin::All => {
                if let Some(origin) = origin_source {
                    Some(origin.to_string())
                } else {
                    Some("*".to_string())
                }
            },
            CorsOrigin::List(origins) => {
                if let Some(origin) = origin_source {
                    match Url::parse(&origin) {
                        Ok(url) => {
                            origins.get(&url.origin()).map(|found_origin| found_origin.ascii_serialization())
                        },
                        Err(_e) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("{}, when parsing {}", _e, origin);
                            None
                        }
                    }
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("could not find origin header in preflight request");
                    None
                }
            }
        };

        if let Some(acao) = acao {
            response.headers.entry("Access-Control-Allow-Origin".to_string()).or_insert_with(|| Vec::new()).push(acao);

            if let Some(max_age) = self.max_age {
                response.headers.entry("Access-Control-Max-Age".to_string()).or_insert_with(|| Vec::new()).push(format!("{}", max_age));
            }
        }
    }

    /// Computed the preflight response
    pub(crate) fn preflight(&self, request: &Request, methods: &HashSet<Method>) -> Response {
        let origin_source = request.headers.get("Origin").map(|o| o.get(0)).flatten().or_else(||
            request.headers.get("origin").map(|o| o.get(0)).flatten()
        );
        let acao = match &self.origins {
            CorsOrigin::None => None,
            CorsOrigin::All => {
                if let Some(origin) = origin_source {
                    Some(origin.to_string())
                } else {
                    Some("*".to_string())
                }
            },
            CorsOrigin::List(origins) => {
                if let Some(origin) = origin_source {
                    match Url::parse(&origin) {
                        Ok(url) => {
                            origins.get(&url.origin()).map(|found_origin| found_origin.ascii_serialization())
                        },
                        Err(_e) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("{}, when parsing {}", _e, origin);
                            None
                        }
                    }
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("could not find origin header in preflight request");
                    None
                }
            }
        };

        if let Some(acao) = acao {
            // Found allowed origin
            let mut response = Response::no_content();

            let methods = match request.headers.get("Access-Control-Request-Method") {
                Some(_) => {
                    if let Some(override_methods) = &self.methods {
                        override_methods.iter()
                    } else {
                        methods.iter()
                    }.map(|m| m.to_str()).collect::<Vec<_>>().join(", ")
                },
                None => {
                    #[cfg(feature = "full_log")]
                    log::debug!("the Access-Control-Request-Method field was not found");
                    return Response::forbidden()
                }
            };

            let headers = if let Some(override_headers) = &self.headers {
                override_headers.iter().cloned().collect::<Vec<_>>().join(", ")
            } else {
                match request.headers.get("Access-Control-Request-Headers").map(|acrh| acrh.get(0)).flatten() {
                    Some(headers) => headers.clone(),
                    None => {
                        #[cfg(feature = "full_log")]
                        log::debug!("the Access-Control-Request-Headers field was not found");
                        return Response::forbidden()
                    }
                }
            };

            #[cfg(feature = "full_log")]
            log::debug!("the preflight request for '{}' is successful, with methods [{}] and headers [{}]", acao, methods, headers);

            response = response.header(
                "Access-Control-Allow-Origin".to_string(),
                acao
            );

            response = response.header(
                "Access-Control-Allow-Methods".to_string(),
                methods
            );

            response = response.header(
                "Access-Control-Allow-Headers".to_string(),
                headers
            );

            if let Some(max_age) = self.max_age {
                response = response.header(
                    "Access-Control-Max-Age".to_string(),
                    format!("{}", max_age)
                );
            }
            response
        } else {
            Response::forbidden()
        }

        /*
        if let Some(origin) = request.headers.get("Origin").or_else(|| request.headers.get("origin")) {
            match Url::parse(&origin) {
                Ok(url) => {
                    let acao = match &self.origins {
                        CorsOrigin::None => None,
                        CorsOrigin::All => Some("*".to_string()),
                        CorsOrigin::List(origins) => {
                            origins.get(&url.origin()).map(|found_origin| found_origin.ascii_serialization())
                        }
                    };

                    if let Some(acao) = acao {
                        // Found allowed origin
                        let mut response = Response::no_content();
                        // It should reply
                        response.headers.insert(
                            "Access-Control-Allow-Origin".to_string(),
                            acao
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
                    log::debug!("{}, when parsing {}", _e, origin);
                }
            }
        }
        Response::forbidden()
        */
    }
}