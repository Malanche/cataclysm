use crate::http::Request;

pub trait Extractor: Send {
    fn extract(req: &Request) -> Self;
}

impl Extractor for Vec<u8> {
    fn extract(req: &Request) -> Self {
        req.content.clone()
    }
}