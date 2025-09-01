#[derive(Debug)]
pub enum Error {
    Builder(usql::builder::Error),
    Unknown(Box<dyn core::error::Error + Send + Sync>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Builder(err) => write!(f, "Builder error: {}", err),
            Error::Unknown(err) => write!(f, "Unknown error: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Builder(err) => Some(err),
            Error::Unknown(err) => Some(&**err),
        }
    }
}

impl Error {
    pub fn new<E: Into<Box<dyn core::error::Error + Send + Sync>>>(error: E) -> Error {
        Error::Unknown(error.into())
    }
}

impl From<usql::builder::Error> for Error {
    fn from(value: usql::builder::Error) -> Self {
        Error::Builder(value)
    }
}
