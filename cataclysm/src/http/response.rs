use std::collections::HashMap;
use crate::Error;

/// Contains the data of an http response
pub struct Response {
    protocol: String,
    pub(crate) status: (u32, String),
    pub(crate) headers: HashMap<String, Vec<String>>,
    pub content: Vec<u8>
}

impl<A: Into<String>> From<(u32, A)> for Response {
    fn from(source: (u32, A)) -> Response {
        Response {
            protocol: "HTTP/1.1".into(),
            status: (source.0, source.1.into()),
            headers: HashMap::new(),
            content: Vec::new()
        }
    }
}

impl Response {
    // Informational
    const CONTINUE: (u32, &'static str) = (100, "Continue");
    const SWITCHING_PROTOCOLS: (u32, &'static str) = (101, "Switching Protocols");

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

    /// Creates an Continue response, with a 100 status code
    pub fn r#continue() -> Response{ Response::CONTINUE.into() }
    /// Creates an Switching Protocols response, with a 101 status code
    pub fn switching_protocols() -> Response{ Response::SWITCHING_PROTOCOLS.into() }

    /// Creates an Ok response, with a 200 status code
    pub fn ok() -> Response { Response::OK.into() }
    /// Creates a Created response, with a 201 status code
    pub fn created() -> Response { Response::CREATED.into() }
    /// Creates an Accepted response, with a 202 status code
    pub fn accepted() -> Response { Response::ACCEPTED.into() }
    /// Creates a Non-Authoritative Information response, with a 203 status code
    pub fn non_authoritative_information() -> Response { Response::NON_AUTHORITATIVE_INFORMATION.into() }
    /// Creates a No Content response, with a 204 status code
    pub fn no_content() -> Response { Response::NO_CONTENT.into() }
    /// Creates a Reset Content response, with a 205 status code
    pub fn reset_content() -> Response { Response::RESET_CONTENT.into() }
    /// Creates a Partial Content response, with a 206 status code
    pub fn partial_content() -> Response { Response::PARTIAL_CONTENT.into() }

    /// Creates a Bad Request response, with a 400 status code
    pub fn bad_request() -> Response { Response::BAD_REQUEST.into() }
    /// Creates an Unauthorized response, with a 401 status code
    pub fn unauthorized() -> Response { Response::UNAUTHORIZED.into() }
    /// Creates a Payment Required response, with a 402 status code
    pub fn payment_required() -> Response { Response::PAYMENT_REQUIRED.into() }
    /// Creates a Forbidden response, with a 403 status code
    pub fn forbidden() -> Response { Response::FORBIDDEN.into() }
    /// Creates a Not Found response, with a 404 status code
    pub fn not_found() -> Response { Response::NOT_FOUND.into() }

    /// Creates an Internal Server Error response, with a 500 status code
    pub fn internal_server_error() -> Response { Response::INTERNAL_SERVER_ERROR.into() }
    /// Creates a Not Implemented response, with a 501 status code
    pub fn not_implemented() -> Response { Response::NOT_IMPLEMENTED.into() }
    /// Creates a Bad Gateway response, with a 502 status code
    pub fn bad_gateway() -> Response { Response::BAD_GATEWAY.into() }
    /// Creates a Service Unavailable response, with a 503 status code
    pub fn service_unavailable() -> Response { Response::SERVICE_UNAVAILABLE.into() }

    /// Creates a new response, with defaut response status 200, and a text/html content type
    pub fn new() -> Response {
        Response::OK.into()
    }

    /// Inserts a header into the response
    pub fn header<A: Into<String>, B: Into<String>>(mut self, key: A, value: B) -> Response {
        self.headers.entry(key.into()).or_insert_with(|| Vec::new()).push(value.into());
        self
    }

    /// Inserts a body in the response
    pub fn body<T: AsRef<[u8]>>(mut self, body: T) -> Response {
        self.content = Vec::from(body.as_ref());
        self
    }

    /// Returns the status code contained in the response
    pub fn status_code(&self) -> u32 {
        self.status.0
    }

    /// Serializes the response to be sent to the client
    pub(crate) fn serialize(&mut self) -> Vec<u8> {
        let mut response = format!("{} {} {}\r\n", self.protocol, self.status.0, self.status.1);

        self.headers.entry("Content-Length".to_string()).or_insert_with(|| Vec::new()).push(format!("{}", self.content.len()));
        for (header_name, headers) in &self.headers {
            for header in headers {
                response += &format!("{}: {}\r\n", header_name, header);
            }
        }
        // print response for debug purposes
        #[cfg(feature = "full_log")]
        log::trace!("serializing http repsonse with headers: {}", response);
        // We finalize the headers
        response += "\r\n";
        // And now add the body, if any
        let mut response = response.into_bytes();
        response.extend_from_slice(&self.content);
        response
    }

    pub(crate) fn parse<A: Into<Vec<u8>>>(bytes: A) -> Result<Response, Error> {
        let mut source: Vec<u8> = bytes.into();

        // http call should have at least 3 bytes. For sure
        let (one, two) = (source.iter(), source.iter().skip(2));

        let mut split_index = None;
        for (idx, (a, b)) in one.zip(two).enumerate() {
            if a==b && b==&b'\n' && idx > 0 && source[idx-1] == b'\r' && source[idx+1] == b'\r' {
                split_index = Some(idx);
                break;
            }
        }

        let split_index = split_index.ok_or(Error::Parse(format!("no end of header was found")))?;

        // The minus one is a safe operation, due to the upper for loop
        let mut content: Vec<_> = source.drain((split_index - 1)..).collect();
        // We have to remove the `\r\n\r\n` that is at the beginning of the remaining bytes
        content.drain(..4);
        // The request header needs to be a string
        let response_string = String::from_utf8(source).map_err(|e| Error::Parse(format!("{}", e)))?;

        let mut lines = response_string.split("\r\n");
        let first_line = lines.next().ok_or(Error::Parse("response has no first line".into()))?;
        let tokens = first_line.split(" ").collect::<Vec<_>>();
        let (protocol, code, status_text) = if tokens.len() < 3 {
            return Err(Error::Parse("responses's first has incorrect format".into()));
        } else {
            (
                tokens[0].to_string(),
                tokens[1].parse::<u32>().map_err(|e| Error::custom(format!("{}", e)))?,
                tokens[2..].join(" ")
            )
        };
        // We parse the remaining headers
        let mut headers = HashMap::new();
        for line in lines {
            let idx = line.find(":").ok_or(Error::Parse(format!("corrupted header missing colon")))?;
            let (key, value) = line.split_at(idx);
            let (key, value) = (key.to_string(), value.trim_start_matches(": ").trim_end().to_string());
            headers.entry(key).or_insert_with(|| Vec::new()).push(value);
        }

        Ok(Response {
            protocol,
            status: (code, status_text),
            headers,
            content
        })
    }
}