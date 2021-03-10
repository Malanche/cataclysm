use std::future::Future;
use std::collections::HashMap;

pub enum Method {
    Get,
    Post,
    Any
}

impl Method {
    pub fn replies_with<F: Fn() -> T, T: Future<Output = Response>>(self, handler: F) -> MethodHandler<F, T> {
        MethodHandler {
            method: self,
            handler
        }
    }
}

pub struct MethodHandler<F: Fn() -> T, T: Future<Output = Response>> {
    pub(crate) method: Method,
    pub(crate) handler: F
}

pub struct Response {
    protocol: String,
    status: (u32, &'static str),
    headers: HashMap<String, String>,
    content: Vec<u8>
}

impl Response {
    const OK: (u32, &'static str) = (200, "OK");

    pub fn ok() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::OK,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }

    pub fn new() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::OK,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }

    pub fn header(mut self, key: String, value: String) -> Response {
        self.headers.insert(key, value);
        self
    }

    pub fn body<T: AsRef<[u8]>>(mut self, body: T) -> Response {
        self.content = Vec::from(body.as_ref());
        self
    }

    pub fn serialize(&mut self) -> Vec<u8> {
        let mut response = format!("{} {} {}\r\n", self.protocol, self.status.0, self.status.1);

        self.headers.insert("Content-Length".into(), format!("{}", self.content.len()));
        response += &self.headers.iter().map(|(key, value)| format!("{}: {}", key, value)).collect::<Vec<_>>().join("\r\n");
        // We finalize the headers
        response += "\r\n\r\n";
        // And now add the body, if any
        let mut response = response.into_bytes();
        response.extend_from_slice(&self.content);
        response
    }
}