use crate::{http::Request, Error, additional::Additional};
use std::sync::Arc;

/// Extractor trait
///
/// You could, if you wish, implement your own Extractor for other classes, which will allow you to construct an instance of `Self` from the `Request`, and from the additional information provided to this call through the Additional structure. The Extractor takes place during the request processing steps when the callback contains such extractor as argument.
pub trait Extractor<T: Sync>: Send + Sized + 'static {
    /// Extract function, constructs Self from the request
    fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error>;
}

impl<T: Sync> Extractor<T> for Vec<u8> {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        Ok(req.content.clone())
    }
}

impl<T: Sync> Extractor<T> for String {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        Ok(String::from_utf8(req.content.clone()).map_err(|e| Error::ExtractionBR(format!("{}", e)))?)
    }
}

impl<T: Sync> Extractor<T> for Request {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        Ok(req.clone())
    }
}

// Implementation for empty tupple, functions with no arguments
impl<T: Sync> Extractor<T> for () {
    fn extract(_req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        Ok(())
    }
}

/// This macro implements the trait for a given indexed tuple, that, as you can see
/// consist in calling the extract method for each element in the tupple
macro_rules! tuple_extractor {
    ($struct_name:ident) => {
        impl<$struct_name, T: Sync> Extractor<T> for ($struct_name,) where $struct_name: Extractor<T> {
            fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
                Ok(($struct_name::extract(req, additional)?,))
            }
        }
    };
    ($($struct_name:ident),+) => {
        impl<$($struct_name),+, T: Sync> Extractor<T> for ($($struct_name),+) where $($struct_name: Extractor<T>),+ {
            fn extract(req: &Request, additional: Arc<Additional<T>>) -> Result<Self, Error> {
                Ok(($($struct_name::extract(req, additional.clone())?),+))
            }
        }
    }
}

tuple_extractor!(A);
tuple_extractor!(A, B);
tuple_extractor!(A, B, C);
tuple_extractor!(A, B, C, D);
tuple_extractor!(A, B, C, D, E);