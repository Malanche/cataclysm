use crate::{
    Error,
    http::{Request, Response},
    session::Session
};
use std::collections::HashMap;

/// Helper trait to give some flexibility to session creation
pub trait SessionCreator: Send + Sync {
    /// Main method, takes the request and should build the session
    ///
    /// In case you are trying to implement this trait yourself, you can use the crate's `custom` method from the `Error` num to give more information if a failure occurs, for debugging porpuses
    fn create(&self, req: &Request) -> Result<Session, Error>;
    fn apply(&self, values: &HashMap<String, String>, res: Response) -> Response;
}