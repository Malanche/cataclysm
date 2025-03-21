use crate::{http::Method};
use super::{RequestHeader};
use url::Url;

/// Contains the data from an http request.
#[derive(Clone, Debug)]
pub struct Request {
    pub(crate) header: RequestHeader,
    pub(crate) content: Vec<u8>
}

impl Request {
    /// Returns the [Url](https://docs.rs/url/latest/url/struct.Url.html) object for this request
    pub fn method(&self) -> &Method {
        &self.header.method
    }
    
    /// Returns the [Url](https://docs.rs/url/latest/url/struct.Url.html) object for this request
    pub fn url(&self) -> &Url {
        &self.header.url
    }

    /// Returns the IP address from which this call has been made
    pub fn address(&self) -> std::net::SocketAddr {
        self.header.addr
    }

    /// Returns the body as bytes of the content
    pub fn body(&self) -> &Vec<u8> {
        &self.content
    }
}