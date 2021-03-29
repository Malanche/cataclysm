use std::collections::HashMap;

pub struct Response {
    protocol: String,
    pub(crate) status: (u32, &'static str),
    headers: HashMap<String, String>,
    content: Vec<u8>
}

impl Into<Response> for (u32, &'static str) {
    fn into(self) -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: self,
            headers: vec![("content-type", "text/html")].into_iter().map(|(a,b)| (a.into(), b.into())).collect(),
            content: Vec::new()
        }
    }
}

impl Response {
    // Successful responses
    const OK: (u32, &'static str) = (200, "OK");
    const CREATED: (u32, &'static str) = (201, "Created");
    const ACCEPTED: (u32, &'static str) = (202, "Accepted");
    const NON_AUTHORITATIVE_INFORMATION: (u32, &'static str) = (203, "Non-Authoritative Information");
    const NO_CONTENT: (u32, &'static str) = (204, "No Content");
    const RESET_CONTENT: (u32, &'static str) = (205, "Reset Content");
    const PARTIAL_CONTENT: (u32, &'static str) = (206, "Partial Content");

    // Redirection Messages

    // Client error responses
    const BAD_REQUEST: (u32, &'static str) = (400, "Bad Request");
    const UNAUTHORIZED: (u32, &'static str) = (401, "Unauthorized");
    const PAYMENT_REQUIRED: (u32, &'static str) = (402, "Payment Required");
    const FORBIDDEN: (u32, &'static str) = (403, "Forbidden");
    const NOT_FOUND: (u32, &'static str) = (404, "Not Found");

    // Server error responses
    const INTERNAL_SERVER_ERROR: (u32, &'static str) = (500, "Internal Server Error");
    const NOT_IMPLEMENTED: (u32, &'static str) = (501, "Not Implemented");
    const BAD_GATEWAY: (u32, &'static str) = (502, "Bad Gateway");
    const SERVICE_UNAVAILABLE: (u32, &'static str) = (503, "Service Unavailable");

    pub fn ok() -> Response { Response::OK.into() }
    pub fn created() -> Response { Response::CREATED.into() }
    pub fn accepted() -> Response { Response::ACCEPTED.into() }
    pub fn non_authoritative_information() -> Response { Response::NON_AUTHORITATIVE_INFORMATION.into() }
    pub fn no_content() -> Response { Response::NO_CONTENT.into() }
    pub fn reset_content() -> Response { Response::RESET_CONTENT.into() }
    pub fn partial_content() -> Response { Response::PARTIAL_CONTENT.into() }

    pub fn bad_request() -> Response { Response::BAD_REQUEST.into() }
    pub fn unauthorized() -> Response { Response::UNAUTHORIZED.into() }
    pub fn payment_required() -> Response { Response::PAYMENT_REQUIRED.into() }
    pub fn forbidden() -> Response { Response::FORBIDDEN.into() }
    pub fn not_found() -> Response { Response::NOT_FOUND.into() }

    pub fn internal_server_error() -> Response { Response::INTERNAL_SERVER_ERROR.into() }
    pub fn not_implemented() -> Response { Response::NOT_IMPLEMENTED.into() }
    pub fn bad_gateway() -> Response { Response::BAD_GATEWAY.into() }
    pub fn service_unavailable() -> Response { Response::SERVICE_UNAVAILABLE.into() }

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