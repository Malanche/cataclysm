use crate::{
    Error,
    http::{Request, Response}
};

/// Helper trait to give some flexibility to session creation
pub trait SessionCreator: Send + Sync {
    /// Main method, takes the cookie string and returns the content, in case it is valid
    ///
    /// In case you are trying to implement this trait yourself, you can use the crate's `custom` method from the `Error` num to give more information if a failure occurs, for debugging porpuses
    fn parse(&self, req: &Request) -> Result<Option<String>, Error>;
    /// Secondary method, applies whatever is required to save the session
    ///
    /// The response is provided in case you require to alter the response (applies for JWT or Cookie sessions). The values of the session are also provided (as they might be used to create a signature). In case you are trying to implement this trait yourself, you can use the crate's `custom` method from the `Error` num to give more information if a failure occurs, for debugging porpuses
    fn apply(&self, content: String, res: Response) -> Result<Response, Error>;
}