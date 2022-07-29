use ring::{hmac::{self, Key}, rand};
use crate::{
    Error,
    http::{Request, Response},
    session::{SessionCreator, Session}
};
use std::collections::HashMap;
use std::time::Duration;
use cookie::Cookie;
use chrono::{DateTime, Utc};

/// Enum to indicate a same site policy in the cookie builder
#[derive(Clone)]
pub enum SameSite {
    Strict,
    Lax,
    None
}

impl SameSite {
    fn to_cookie_same_site(&self) -> cookie::SameSite {
        match self {
            SameSite::Strict => cookie::SameSite::Strict,
            SameSite::Lax => cookie::SameSite::Lax,
            SameSite::None => cookie::SameSite::None
        }
    }
}

#[derive(Clone)]
pub struct CookieSession {
    key: Key,
    cookie_name: String,
    path: Option<String>,
    domain: Option<String>,
    expires: Option<DateTime<Utc>>,
    max_age: Option<Duration>,
    secure: Option<bool>,
    http_only: Option<bool>,
    same_site: Option<SameSite>,
    force_failure: bool
}

impl CookieSession {
    /// Creates a new cookie session creator
    pub fn new() -> Self {
        let rng = rand::SystemRandom::new();
        CookieSession {
            key: Key::generate(hmac::HMAC_SHA256, &rng).map_err(Error::Ring).unwrap(),
            cookie_name: "cataclysm-session".to_string(),
            path: None,
            domain: None,
            expires: None,
            max_age: None,
            secure: None,
            http_only: None,
            same_site: None,
            force_failure: false
        }
    }

    /// Sets a custom `name` for the cookie.
    ///
    /// By default, `cataclysm-session` is used.
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new()
    ///     .name("my-app-session");
    /// ```
    pub fn name<A: Into<String>>(mut self, name: A) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Sets a custom `Key` for cookie signature
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new()
    ///     .secret("really secret!");
    /// ```
    ///
    /// If no secret is provided, a random key will be used (generated by ring).
    pub fn secret<A: AsRef<[u8]>>(mut self, secret: A) -> Self {
        self.key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_ref());
        self
    }

    /// Sets a custom `path` for generated cookies
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new()
    ///     .path("/applies/only/here");
    /// ```
    pub fn path<A: Into<String>>(mut self, path: A) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets a custom `domain` for generated cookies
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new()
    ///     .domain("example.com");
    /// ```
    pub fn domain<A: Into<String>>(mut self, domain: A) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Sets a duration for the cookie (that is, using the `expires` field)
    ///
    /// You should try to avoid this method, and use `max_age` instead.
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// use chrono::{DateTime, Utc, Duration};
    /// 
    /// let cookie_session = CookieSession::new().expires(Utc::now() + Duration::weeks(52));
    /// ```
    ///
    /// Please note that the "expires" field will not update the provided date as time goes by. Use the [max_age](CookieSession::max_age) method instead.
    pub fn expires(mut self, expires: DateTime<Utc>) -> Self {
        self.expires = Some(expires);
        self
    }

    /// Sets a duration for the cookie (that is, using the `max_age` field)
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// use std::time::Duration;
    /// 
    /// let cookie_session = CookieSession::new().max_age(Duration::from_secs(3_600 * 6));
    /// ```
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// Sets the `secure` field in the cookie
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new().secure(true);
    /// ```
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = Some(secure);
        self
    }

    /// Sets the `http_only` field in the cookie
    ///
    /// ```rust,no_run
    /// use cataclysm::session::CookieSession;
    /// 
    /// let cookie_session = CookieSession::new().http_only(true);
    /// ```
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = Some(http_only);
        self
    }

    /// Sets the `same_site` field in the cookie
    ///
    /// ```rust,no_run
    /// use cataclysm::session::{CookieSession, SameSite};
    /// 
    /// let cookie_session = CookieSession::new().same_site(SameSite::Lax);
    /// ```
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }

    /// Helper function to extract a session from a cookie
    fn build_from_req(&self, req: &Request) -> Result<Option<Session>, Error> {
        if let Some(cookie_string) = req.headers.get("Cookie").or_else(|| req.headers.get("cookie")) {
            let cookie = Cookie::parse_encoded(cookie_string).map_err(|e| Error::custom(format!("{}", e)))?;
            let value = cookie.value();
            // The hmac value is at least 44 bytes
            if value.len() < 44 {
                return Err(Error::custom("length of cookie cannot contain even the hmac value"));
            } else {
                // I know these unwraps look unsafe, but trust me, they are, TODO FIX with let Some
                let signature = value.get(0..44).unwrap();
                let content = value.get(44..value.len()).unwrap();

                // First, we try to decode the content
                let values = serde_json::from_str(content).map_err(|e| Error::custom(format!("{}", e)))?;

                let tag = base64::decode(signature).map_err(|e| Error::custom(format!("{}", e)))?;

                hmac::verify(&self.key, content.as_bytes(), &tag).map_err(|e| Error::custom(format!("{}", e)))?;

                Ok(Some(Session::new_with_values(self.clone(), values)))
            }
        } else {
            Ok(None)
        }
    }
}

impl SessionCreator for CookieSession {
    fn create(&self, req: &Request) -> Result<Session, Error> {
        match self.build_from_req(req) {
            Ok(Some(session)) => Ok(session),
            Ok(None) => {
                #[cfg(feature = "full_log")]
                log::debug!("cookie not found among request headers");
                Ok(Session::new(self.clone()))
            },
            Err(e) => {
                if self.force_failure {
                    Err(e)
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("error while creating session: {}", e);
                    return Ok(Session::new(self.clone()))
                }
            }
        }
    }

    fn apply(&self, values: &HashMap<String, String>, mut res: Response) -> Response {
        let content = serde_json::to_string(values).unwrap();
        let signature = base64::encode(hmac::sign(&self.key, content.as_bytes()).as_ref());

        let cookie_builder = Cookie::build(&self.cookie_name, format!("{}{}", signature, content));

        let cookie_builder = if let Some(path) = &self.path {
            cookie_builder.path(path)
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(domain) = &self.domain {
            cookie_builder.domain(domain)
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(expires) = &self.expires {
            match cookie::time::OffsetDateTime::from_unix_timestamp(expires.timestamp()) {
                Ok(v) => cookie_builder.expires(v),
                Err(e) => {
                    log::error!("could not set expires flag to cookie, {}", e);
                    cookie_builder
                }
            }
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(max_age) = &self.max_age {
            let max_age = match max_age.clone().try_into() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("failed to set max-age to cookie ({}), loading safety default 3,600 seconds", e);
                    cookie::time::Duration::seconds(3_600)
                }
            };
            cookie_builder.max_age(max_age)
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(secure) = &self.secure {
            cookie_builder.secure(*secure)
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(http_only) = &self.http_only {
            cookie_builder.http_only(*http_only)
        } else {
            cookie_builder
        };

        let cookie_builder = if let Some(same_site) = &self.same_site {
            cookie_builder.same_site(same_site.to_cookie_same_site())
        } else {
            cookie_builder
        };

        let cookie = cookie_builder.finish();

        res.headers.insert("Set-Cookie".to_string(), format!("{}", cookie.encoded()));
        res
    }
}