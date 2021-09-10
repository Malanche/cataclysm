use crate::{Extractor, Error, http::Request};
use std::collections::HashMap;
use cookie::Cookie;

pub struct Session {
    values: HashMap<String, String>
}

impl Session {
    fn new() -> Session {
        Session{
            values: HashMap::new()
        }
    }
}

impl Session {
    pub fn set(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }

    pub fn get<T: AsRef<str>>(&self, key: T) -> Option<&String> {
        self.values.get(key.as_ref())
    }
}

impl Extractor for Session {
    fn extract(req: &Request) -> Result<Self, Error> {
        if let Some(cookie_string) = req.headers.get("Cookie") {
            let _cookie = match Cookie::parse(cookie_string) {
                Ok(v) => v,
                Err(_e) => return Ok(Session::new())
            };
            Ok(Session {
                values: HashMap::new()
            })
        } else {
            return Ok(Session::new())
        }
    }
}