#[derive(Clone, Debug)]
pub enum Error {
    Dummy
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::Dummy => format!("Dummy error")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}