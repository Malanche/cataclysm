use crate::{Extractor, Error, http::{Request, Response}, additional::Additional};
use std::collections::HashMap;
use cookie::Cookie;
use std::sync::Arc;
use ring::hmac::{self, Key};

/// Working sessions, but not finished
pub struct Session {
    values: HashMap<String, String>,
    changed: bool,
    secret: Arc<Key>
}

impl Session {
    /// Creates a new session
    fn new(secret: Arc<Key>) -> Session {
        Session{
            values: HashMap::new(),
            changed: false,
            secret
        }
    }
}

impl Session {
    /// Sets a new value in the session
    pub fn set<A: Into<String>, B: Into<String>>(&mut self, key: A, value: B) {
        self.changed = true;
        self.values.insert(key.into(), value.into());
    }

    /// Retrieves a value from the session
    pub fn get<T: AsRef<str>>(&self, key: T) -> Option<&String> {
        self.values.get(key.as_ref())
    }

    /// Clears all values in the session
    pub fn clear(&mut self) {
        self.changed = true;
        self.values.clear();
    }

    /// Applies all the changes of the session to the response.
    ///
    /// It is not the most elegant solution, but soon a new one will be worked out to apply the session changes to the response (probably using layers).
    pub fn apply(self, mut req: Response) -> Response {
        if self.changed {
            let content = serde_json::to_string(&self.values).unwrap();
            let signature = base64::encode(hmac::sign(&self.secret, content.as_bytes()).as_ref());
            let cookie = Cookie::build("cataclysm-session", format!("{}{}", signature, content))
                .path("/").finish();
            req.headers.insert("Set-Cookie".to_string(), format!("{}", cookie.encoded()));
            req
        } else {
            req
        }
    }
}

impl<T: Sync> Extractor<T> for Session {
    fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(cookie_string) = req.headers.get("Cookie") {
            match Cookie::parse_encoded(cookie_string) {
                Ok(cookie) => {
                    let value = cookie.value();
                    if value.len() < 44 {
                        Ok(Session::new(additional.secret.clone()))
                    } else {
                        let signature = value.get(0..44).unwrap();
                        let content = value.get(44..value.len()).unwrap();

                        // First, we try to decode the content
                        match serde_json::from_str(content) {
                            Ok(values) => {
                                match base64::decode(signature) {
                                    Ok(tag) => {
                                        match hmac::verify(&additional.secret, content.as_bytes(), &tag) {
                                            Ok(_) => Ok(Session{
                                                values,
                                                changed: false,
                                                secret: additional.secret.clone()
                                            }),
                                            Err(_) => Ok(Session::new(additional.secret.clone()))
                                        }
                                    },
                                    Err(_) => Ok(Session::new(additional.secret.clone()))
                                }
                            },
                            Err(_) => Ok(Session::new(additional.secret.clone()))
                        }
                    }
                },
                Err(_e) => Ok(Session::new(additional.secret.clone()))
            }
        } else {
            return Ok(Session::new(additional.secret.clone()))
        }
    }
}