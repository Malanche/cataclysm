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

impl Extractor for () {
    fn extract(_req: &Request) -> Self {
        ()
    }
}

impl<A> Extractor for (A,) where A: Extractor {
    fn extract(req: &Request) -> Self {
        (A::extract(req),)
    }
}

impl<A, B> Extractor for (A, B) where A: Extractor, B: Extractor {
    fn extract(req: &Request) -> Self {
        (A::extract(req), B::extract(req))
    }
}