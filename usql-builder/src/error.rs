use core::fmt;

#[derive(Debug)]
pub enum Error {
    Write(fmt::Error),
}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        Error::Write(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write(e) => e.fmt(f),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Write(e) => Some(e),
        }
    }
}
