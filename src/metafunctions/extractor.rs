use crate::http::Request;

/// Extractor trait
///
/// You could, if you wish, implement your own Extractor for other classes, which will allow you to construct an instance of `Self` from the `Request`. The Extractor takes place during the request processing steps when the callback contains such extractor as argument.
pub trait Extractor: Send + 'static {
    /// Extract function
    fn extract(req: &Request) -> Self;
}

impl Extractor for Vec<u8> {
    fn extract(req: &Request) -> Self {
        req.content.clone()
    }
}

impl Extractor for String {
    fn extract(req: &Request) -> Self {
        String::from_utf8(req.content.clone()).unwrap()
    }
}

impl Extractor for Request {
    fn extract(req: &Request) -> Self {
        req.clone()
    }
}

// Implementation for empty tupple, functions with no arguments
impl Extractor for () {
    fn extract(_req: &Request) -> Self {
        ()
    }
}

/// This macro implements the trait for a given indexed tuple
macro_rules! tuple_extractor {
    ($struct_name:ident) => {
        impl<$struct_name> Extractor for ($struct_name,) where $struct_name: Extractor {
            fn extract(req: &Request) -> Self {
                ($struct_name::extract(req),)
            }
        }
    };
    ($($struct_name:ident),+) => {
        impl<$($struct_name),+> Extractor for ($($struct_name),+) where $($struct_name: Extractor),+ {
            fn extract(req: &Request) -> Self {
                ($($struct_name::extract(req)),+)
            }
        }
    }
}

tuple_extractor!(A);
tuple_extractor!(A, B);
tuple_extractor!(A, B, C);
tuple_extractor!(A, B, C, D);
tuple_extractor!(A, B, C, D, E);