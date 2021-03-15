use std::collections::HashMap;

pub struct Response {
    protocol: String,
    pub(crate) status: (u32, &'static str),
    headers: HashMap<String, String>,
    content: Vec<u8>
}

impl Response {
    const OK: (u32, &'static str) = (200, "OK");
    const BAD_REQUEST: (u32, &'static str) = (400, "Bad Request");
    const NOT_FOUND: (u32, &'static str) = (404, "Not Found");
    const INTERNAL_SERVER_ERROR: (u32, &'static str) = (500, "Internal Server Error");

    pub fn ok() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::OK,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }

    pub fn not_found() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::NOT_FOUND,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }

    pub fn bad_request() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::BAD_REQUEST,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }

    pub fn internal_server_error() -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: Response::INTERNAL_SERVER_ERROR,
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