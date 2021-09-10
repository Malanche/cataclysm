use crate::http::Request;
use crate::Error;

/// Extractor trait
///
/// You could, if you wish, implement your own Extractor for other classes, which will allow you to construct an instance of `Self` from the `Request`. The Extractor takes place during the request processing steps when the callback contains such extractor as argument.
pub trait Extractor: Send + Sized + 'static {
    /// Extract function, constructs Self from the request
    fn extract(req: &Request) -> Result<Self, Error>;
}

impl Extractor for Vec<u8> {
    fn extract(req: &Request) -> Result<Self, Error> {
        Ok(req.content.clone())
    }
}

impl Extractor for String {
    fn extract(req: &Request) -> Result<Self, Error> {
        Ok(String::from_utf8(req.content.clone()).map_err(|e| Error::ExtractionBR(format!("{}", e)))?)
    }
}

impl Extractor for Request {
    fn extract(req: &Request) -> Result<Self, Error> {
        Ok(req.clone())
    }
}

// Implementation for empty tupple, functions with no arguments
impl Extractor for () {
    fn extract(_req: &Request) -> Result<Self, Error> {
        Ok(())
    }
}

/// This macro implements the trait for a given indexed tuple, that, as you can see
/// consist in calling the extract method for each element in the tupple
macro_rules! tuple_extractor {
    ($struct_name:ident) => {
        impl<$struct_name> Extractor for ($struct_name,) where $struct_name: Extractor {
            fn extract(req: &Request) -> Result<Self, Error> {
                Ok(($struct_name::extract(req)?,))
            }
        }
    };
    ($($struct_name:ident),+) => {
        impl<$($struct_name),+> Extractor for ($($struct_name),+) where $($struct_name: Extractor),+ {
            fn extract(req: &Request) -> Result<Self, Error> {
                Ok(($($struct_name::extract(req)?),+))
            }
        }
    }
}

tuple_extractor!(A);
tuple_extractor!(A, B);
tuple_extractor!(A, B, C);
tuple_extractor!(A, B, C, D);
tuple_extractor!(A, B, C, D, E);