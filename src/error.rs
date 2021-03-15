#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    Dummy
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Io(inner_error) => format!("io error: {}", inner_error),
            Error::Parse(detail) => format!("parse error: {}", detail),
            Error::Dummy => format!("Dummy error")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}