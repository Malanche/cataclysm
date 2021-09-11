use crate::{Shared};
use ring::hmac::{Key};
use std::sync::Arc;

/// Wrapper for additional shared data in the server
///
/// This structure is reserved for future use, particularly dealing with the pipeline (possibly session management)
pub struct Additional<T> {
    pub(crate) shared: Option<Shared<T>>,
    pub(crate) secret: Arc<Key>
}