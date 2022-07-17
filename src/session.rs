pub use self::session_creator::SessionCreator;
pub use self::cookie_session::CookieSession;
mod session_creator;
mod cookie_session;

use crate::{Extractor, Error, http::{Request, Response}, additional::Additional};
use std::collections::HashMap;
use std::sync::Arc;

/// Working sessions, but not finished
pub struct Session {
    values: HashMap<String, String>,
    changed: bool,
    session_creator: Arc<Box<dyn SessionCreator>>
}

impl Session {
    /// Creates a new session
    fn new<A: 'static + SessionCreator>(session_creator: A) -> Session {
        let session_creator: Arc<Box<dyn SessionCreator>> = Arc::new(Box::new(session_creator));
        Session{
            values: HashMap::new(),
            changed: false,
            session_creator
        }
    }

    /// Creates a new session
    pub fn new_with_values<A: 'static + SessionCreator>(session_creator: A, values: HashMap<String, String>) -> Session {
        let mut session = Session::new(session_creator);
        session.values = values;
        session
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
    pub fn apply(self, req: Response) -> Response {
        if self.changed {
            self.session_creator.apply(&self.values, req)
        } else {
            req
        }
    }
}

impl<T: Sync> Extractor<T> for Session {
    fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(session_creator) = &additional.session_creator {
            session_creator.create(req)
        } else {
            // Forcefully log an error message, as this should be quickly noticed by the developer
            log::error!("cataclysm error: you need to setup a `SessionCreator` before you try to use the `Session` extractor!");
            Err(Error::NoSessionCreator)
        }
    }
}