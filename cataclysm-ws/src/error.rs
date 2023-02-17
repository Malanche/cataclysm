/// Error indicating a frame parsing error
#[derive(Debug)]
pub enum FrameParseError {
    /// Indicates that the first 4 bits of the message are unsupported
    WrongFinRSV,
    /// Indicates that the message content is incomplete
    Incomplete,
    /// Indicates that the message is malformed
    Malformed,
    /// Indicates that the message contains 0 bytes
    NullContent,
    /// The text sent through the message is not valid a utf-8
    InvalidUtf8(std::string::FromUtf8Error),
    /// Indicates an unsupported operation code contained in the frame
    UnsupportedOpCode
}

impl std::fmt::Display for FrameParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            FrameParseError::WrongFinRSV => format!("first 4 bits of the message are malformed"),
            FrameParseError::Incomplete => format!("mismatching payload length in message"),
            FrameParseError::Malformed => format!("the message does not have the corret structure or enough bytes"),
            FrameParseError::NullContent => format!("can't parse because the message has length 0"),
            FrameParseError::InvalidUtf8(e) => format!("invalid utf8 bytes, {}", e),
            FrameParseError::UnsupportedOpCode => format!("the op code received is not supported")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for FrameParseError {}

/// Errors thrown by this library
#[derive(Debug)]
pub enum Error {
    /// Standard io error
    Io(std::io::Error),
    /// Could not parse properly a frame, the detail is contained inside
    FrameParse(FrameParseError),
    /// Indicates that the connection was closed abruptly
    ConnectionReset
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::FrameParse(fpe) => format!("frame parse error: {}", fpe),
            Error::ConnectionReset => format!("connection reset by peer")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}