/// Errors thrown by this library
#[derive(Debug)]
pub enum Error {
    /// Standard io error
    Io(std::io::Error),
    /// Could not parse properly a frame
    Parse(String),
    /// Indicate sthat the connection was closed abruptly
    ConnectionReset,
    /// Indicates a websockets Incomplete Message
    Incomplete
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::Parse(detail) => format!("parse error: {}", detail),
            Error::ConnectionReset => format!("connection reset by peer"),
            Error::Incomplete => format!("incomplete frame message in ws connection"),
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}