use core::fmt;

use usql_value::Type;

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Channel,
    NotFound,
    Pool,
    Convert { found: Option<Type>, expected: Type },
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Sqlite(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "{}", err),
            Self::Channel => write!(f, "channel"),
            Self::NotFound => write!(f, "not found"),
            Self::Pool => write!(f, "pool"),
            Self::Convert { found, expected } => {
                write!(
                    f,
                    "Convert: found: {}, expected {}",
                    found
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    expected
                )
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Sqlite(err) => Some(err),
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
