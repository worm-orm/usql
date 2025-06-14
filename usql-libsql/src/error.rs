use core::fmt;

#[derive(Debug)]
pub enum Error {
    LibSql(libsql::Error),
    NotFound,
    Pool,
}

impl From<libsql::Error> for Error {
    fn from(value: libsql::Error) -> Self {
        Error::LibSql(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibSql(err) => err.fmt(f),
            Self::NotFound => write!(f, "not found"),
            Self::Pool => write!(f, "pool"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::LibSql(err) => Some(err),
            _ => None,
        }
    }
}

impl From<deadpool::managed::PoolError<Error>> for Error {
    fn from(value: deadpool::managed::PoolError<Error>) -> Self {
        match value {
            deadpool::managed::PoolError::Backend(e) => e,
            _ => Error::Pool,
        }
    }
}
