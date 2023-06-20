use crate::{Shared};
use std::sync::Arc;
use crate::session::SessionCreator;

/// Wrapper for additional shared data in the server
///
/// This structure is reserved for future use, particularly dealing with the pipeline (possibly session management)
pub struct Additional<T> {
    pub(crate) shared: Option<Shared<T>>,
    pub(crate) session_creator: Option<Arc<Box<dyn SessionCreator>>>,
}

impl<T> Additional<T> {
    /// Gives back a copy of the contained `Shared` object, in case there is some
    pub fn shared(&self) -> Option<Shared<T>> {
        self.shared.clone()
    }
}