use core::fmt;

use alloc::string::String;

#[derive(Debug)]
pub enum Error {
    Write(fmt::Error),
    InvalidAutoType(String),
    InvalidValueCount { expected: usize, found: usize },
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
            Self::InvalidAutoType(ty) => write!(f, "Invalid auto type: {}", ty),
            Self::InvalidValueCount { expected, found } => {
                write!(
                    f,
                    "Invalid value count. Found: {}, expected: {}",
                    expected, found
                )
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Write(e) => Some(e),
            _ => None,
        }
    }
}
