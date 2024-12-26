#[cfg(feature = "full_log")]
use crate::http::Response;

/// Errors returned by this library
#[derive(Debug)]
pub enum Error {
    /// Standard io error
    Io(std::io::Error),
    /// Could not parse properly the http request, malformed
    Parse(String),
    /// Waiting time for the client got exceeded
    Timeout,
    /// Error from url parsing
    Url(url::ParseError),
    /// Could not extract parameter from request. Indicating a bad request.
    ExtractionBR(String),
    /// Could not extract parameter from request. Indicating a bad server error.
    ExtractionSE(String),
    /// Indicates a Ring error
    Ring(ring::error::Unspecified),
    /// Indicates that no session creator was set
    NoSessionCreator,
    /// Custom error, try to avoid its use
    Custom(String)
}

#[cfg(feature = "full_log")]
#[derive(serde::Serialize)]
struct ErrorResponse {
    detail: String
}

impl Error {
    /// Creates a custom error with a custom message
    pub fn custom<A: Into<String>>(message: A) -> Error {
        Error::Custom(message.into())
    }

    #[cfg(feature = "full_log")]
    pub fn as_response(&self) -> Response {
        let (mut base_response, content) = match self {
            Error::Io(e) => (Response::internal_server_error(), ErrorResponse{detail: format!("{}", e)}),
            Error::Parse(e) => (Response::bad_request(), ErrorResponse{detail: e.to_string()}),
            Error::Timeout => (Response::bad_request(), ErrorResponse{detail: format!("timeout reached")}),
            Error::Url(e) => (Response::bad_request(), ErrorResponse{detail: format!("{}", e)}),
            Error::ExtractionBR(e) => (Response::bad_request(), ErrorResponse{detail: e.to_string()}),
            Error::ExtractionSE(e) => (Response::internal_server_error(), ErrorResponse{detail: e.to_string()}),
            Error::Ring(ring::error::Unspecified) => (Response::internal_server_error(), ErrorResponse{detail: "no detail".to_string()}),
            Error::NoSessionCreator => (Response::internal_server_error(), ErrorResponse{detail: "missconfiguration".to_string()}),
            Error::Custom(e) => (Response::internal_server_error(), ErrorResponse{detail: e.to_string()})
        };

        let content = match serde_json::to_string(&content) {
            Ok(v) => v,
            Err(_) => {
                base_response = Response::internal_server_error();
                format!("{{\"detail\": \"serialization\"}}")
            }
        };

        base_response.header("Content-Type", "application/json").body(content)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::Parse(detail) => format!("parse error: {}", detail),
            Error::Timeout => format!("timeout reached"),
            Error::Url(detail) => format!("url parse error: {}", detail),
            Error::ExtractionBR(detail) => format!("extraction bad request: {}", detail),
            Error::ExtractionSE(detail) => format!("extraction server error: {}", detail),
            Error::Ring(e) => format!("ring error: {}", e),
            Error::NoSessionCreator => format!("the session extractor requires a SessionCreator struct to work, see documentation"),
            Error::Custom(e) => format!("{}", e)
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}