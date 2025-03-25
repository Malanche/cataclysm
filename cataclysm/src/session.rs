pub use self::session_creator::SessionCreator;
pub use self::cookie_session::{CookieSession, SameSite};
pub use self::session_parser::{SessionParser};
pub use self::json_session_parser::{JsonSessionParser};

mod session_creator;
mod cookie_session;
mod session_parser;
mod json_session_parser;

use crate::{Extractor, Error, http::{Request, Response}, additional::Additional};
use std::sync::Arc;
use serde::{Serialize, de::{DeserializeOwned}};

/// More advanced session, with strongly typed functionality
///
/// Sessions, by default, store values in a one-level string to string json, which is represented by a [HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html).
/// In order to deserialize any oder kind of structure, you can specify the first generic argument of the [Session] struct.
///
/// ```rs,no_run
/// 
/// ``` 
pub struct Session<B = std::collections::HashMap<String, String>, P = JsonSessionParser> {
    inner: Option<B>,
    phantom: std::marker::PhantomData<P>,
    session_creator: Arc<Box<dyn SessionCreator>>
}

impl<B, P> Session<B, P> {
    /// Creates a new session
    fn new(session_creator: Arc<Box<dyn SessionCreator>>) -> Session<B, P> {
        Session{
            inner: None,
            phantom: std::marker::PhantomData,
            session_creator
        }
    }
}

impl<B, P> std::ops::Deref for Session<B, P> {
    type Target = Option<B>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B, P> std::ops::DerefMut for Session<B, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<B: Send + Serialize, P: SessionParser<B>> Session<B, P> {
    /// Applies all the changes of the session to the response.
    ///
    /// It is not the most elegant solution, but soon a new one will be worked out to apply the session changes to the response (probably using layers).
    pub fn apply(self, req: Response) -> Result<Response, Error> {
        let content = serde_json::to_string(&self.inner)?;
        self.session_creator.apply(content, req)
    }
}

impl<T: Sync, B: 'static + Send + DeserializeOwned, P: 'static + Send + SessionParser<B>> Extractor<T> for Session<B, P> {
    fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(session_creator) = &additional.session_creator {
            match session_creator.parse(req)? {
                Some(content) => {
                    let inner = serde_json::from_str(&content).map_err(|e| Error::ExtractionBR(format!("{}", e)))?;
                    Ok(Session {
                        inner,
                        phantom: std::marker::PhantomData,
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