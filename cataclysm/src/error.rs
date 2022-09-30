/// Errors returned by this library
#[derive(Debug)]
pub enum Error {
    /// Standard io error
    Io(std::io::Error),
    /// Could not parse properly the http request, malformed
    Parse(String),
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

impl Error {
    /// Creates a custom error with a custom message
    pub fn custom<A: Into<String>>(message: A) -> Error {
        Error::Custom(message.into())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::Parse(detail) => format!("parse error: {}", detail),
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