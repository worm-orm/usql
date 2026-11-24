use core::fmt;

use deadpool_postgres::{ConfigError, CreatePoolError, PoolError};
use usql_value::Type;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Postgres(tokio_postgres::Error),
    Pool(PoolError),
    CreatePool(CreatePoolError),
    #[cfg(feature = "rustls")]
    Tls(Box<dyn core::error::Error + Send + Sync>),
    NotFound,
    Convert {
        found: Option<Type>,
        expected: Type,
    },
}

impl From<CreatePoolError> for Error {
    fn from(value: CreatePoolError) -> Self {
        Error::CreatePool(value)
    }
}

impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Error::CreatePool(CreatePoolError::Config(value))
    }
}

impl From<PoolError> for Error {
    fn from(value: PoolError) -> Self {
        match value {
            PoolError::Backend(err) => Error::Postgres(err),
            err => Error::Pool(err),
        }
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(value: tokio_postgres::Error) -> Self {
        Error::Postgres(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The error itself only says "db error"
            Self::Postgres(err) => match err.as_db_error() {
                Some(db) => write!(f, "{db}"),
                None => write!(f, "{err}"),
            },
            Self::Pool(err) => write!(f, "{err}"),
            Self::CreatePool(err) => write!(f, "{err}"),
            #[cfg(feature = "rustls")]
            Self::Tls(err) => write!(f, "tls: {err}"),
            Self::NotFound => write!(f, "not found"),
            Self::Convert { found, expected } => match found {
                Some(found) => write!(f, "Convert: found: {found}, expected {expected}"),
                None => write!(f, "Convert: found: null, expected {expected}"),
            },
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Postgres(err) => Some(err),
            Self::Pool(err) => Some(err),
            Self::CreatePool(err) => Some(err),
            #[cfg(feature = "rustls")]
            Self::Tls(err) => Some(&**err),
            _ => None,
        }
    }
}
