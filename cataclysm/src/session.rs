pub use self::session_creator::SessionCreator;
pub use self::cookie_session::{CookieSession, SameSite};
mod session_creator;
mod cookie_session;

use crate::{Extractor, Error, http::{Request, Response}, additional::Additional};
use std::sync::Arc;
use serde::{Serialize, de::{DeserializeOwned}};

/// More advanced session, with strongly typed functionality
pub struct Session<B = std::collections::HashMap<String, String>> {
    inner: Option<B>,
    session_creator: Arc<Box<dyn SessionCreator>>
}

impl<B> Session<B> {
    /// Creates a new session
    fn new(session_creator: Arc<Box<dyn SessionCreator>>) -> Session<B> {
        Session{
            inner: None,
            session_creator
        }
    }
}

impl<B> std::ops::Deref for Session<B> {
    type Target = Option<B>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B> std::ops::DerefMut for Session<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<B: Send + Serialize> Session<B> {
    /// Applies all the changes of the session to the response.
    ///
    /// It is not the most elegant solution, but soon a new one will be worked out to apply the session changes to the response (probably using layers).
    pub fn apply(self, req: Response) -> Result<Response, Error> {
        let content = serde_json::to_string(&self.inner)?;
        self.session_creator.apply(content, req)
    }
}

impl<T: Sync, B: 'static + Send + DeserializeOwned> Extractor<T> for Session<B> {
    fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(session_creator) = &additional.session_creator {
            match session_creator.parse(req)? {
                Some(content) => {
                    let inner = serde_json::from_str(&content).map_err(|e| Error::ExtractionBR(format!("{}", e)))?;
                    Ok(Session {
                        inner,
                        session_creator: session_creator.clone()
                    })
                },
                None => Ok(Session::new(session_creator.clone()))
            }
        } else {
            // Forcefully log an error message, as this should be quickly noticed by the developer
            log::error!("cataclysm error: you need to setup a `SessionCreator` before you try to use the `Session` extractor!");
            Err(Error::NoSessionCreator)
        }
    }
}