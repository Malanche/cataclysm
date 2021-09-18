use crate::{Extractor, Error, Additional, http::{Request}};
use std::sync::Arc;
use std::ops::{Deref};

/// Wrapper around data to be shared in the server
///
/// The main use for this structure is to work as an extractor in the server callbacks to access shared data. An example can be found in the ServerBuilder's [share](crate::ServerBuilder::share).
pub struct Shared<T> {
    inner: Arc<T>
}

// Convenience deref and deref mut
impl<T> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared {
            inner: self.inner.clone()
        }
    }
}

impl<T> Shared<T> {
    /// Creates a new shared instance
    pub(crate) fn new(inner: T) -> Shared<T> {
        Shared {
            inner: Arc::new(inner)
        }
    }

    /// Extracts the contained data from the `Shared` in an `Arc`
    pub fn into_inner(self) -> Arc<T> {
        self.inner
    }
}

impl<T: 'static + Sync + Send> Extractor<T> for Shared<T> {
    fn extract(_req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(shared) = &additional.shared {
            Ok((*shared).clone())
        } else {
            Err(Error::ExtractionSE(format!("No shared was set up by the server...")))
        }
    }
}