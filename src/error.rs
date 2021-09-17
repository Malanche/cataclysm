/// Errors returned by this library
#[derive(Debug)]
pub enum Error {
    /// Standard io error
    Io(std::io::Error),
    /// Could not parse properly the http request, malformed
    Parse(String),
    /// Could not extract parameter from request. Indicating a bad request.
    ExtractionBR(String),
    /// Could not extract parameter from request. Indicating a bad server error.
    ExtractionSE(String),
    /// Indicates a Ring error
    Ring,
    /// Indicates that no gate was provided to spawn demons
    #[cfg(feature = "demon")]
    MissingGate,
    /// Internal error in apocalypse
    #[cfg(feature = "demon")]
    Apocalypse(apocalypse::Error),
    /// Dummy error, needs to be removed
    Dummy
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::Parse(detail) => format!("parse error: {}", detail),
            Error::ExtractionBR(detail) => format!("extraction bad request: {}", detail),
            Error::ExtractionSE(detail) => format!("extraction server error: {}", detail),
            Error::Ring => format!("ring error"),
            #[cfg(feature = "demon")]
            Error::MissingGate => format!("for demon spawning, a gate must be provided to the server through the builder"),
            #[cfg(feature = "demon")]
            Error::Apocalypse(inner_error) => format!("apocalypse error, {}", inner_error),
            Error::Dummy => format!("Dummy error")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}