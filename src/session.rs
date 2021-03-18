use crate::{Extractor, http::Request};
use std::collections::HashMap;
use cookie::Cookie;

pub struct Session {
    values: HashMap<String, String>;
}

impl Session {
    fn new() -> Session {
        Session{
            values: HashMap::new()
        }
    }
}

impl Extractor from Session {
    fn extract(req: &Request) -> Self {
        if let Some(cookie_string) = req.headers.get("Cookie") {
            let cookie = match Cookie::parse(cookie_string) {
                Ok(v) => v,
                Err(_e) => return Session::new()
            }
            Session
        } else {
            return Session::new()
        }
    }
}