use core::fmt;

use usql_value::Type;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Mysql(mysql_async::Error),
    NotFound,
    Convert {
        found: Option<Type>,
        expected: Type,
    },
    /// A value that can't be sent to MySQL
    Unsupported(&'static str),
    /// A value from MySQL that can't be represented, like the zero date
    Decode(&'static str),
    /// The number of parameters doesn't match the statement
    ParamCount {
        expected: usize,
        found: usize,
    },
}

impl From<mysql_async::Error> for Error {
    fn from(value: mysql_async::Error) -> Self {
        Error::Mysql(value)
    }
}

impl From<mysql_async::UrlError> for Error {
    fn from(value: mysql_async::UrlError) -> Self {
        Error::Mysql(value.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mysql(err) => write!(f, "{err}"),
            Self::NotFound => write!(f, "not found"),
            Self::Convert { found, expected } => match found {
                Some(found) => write!(f, "Convert: found: {found}, expected {expected}"),
                None => write!(f, "Convert: found: null, expected {expected}"),
            },
            Self::Unsupported(what) => write!(f, "unsupported by mysql: {what}"),
            Self::Decode(what) => write!(f, "decode: {what}"),
            Self::ParamCount { expected, found } => {
                write!(f, "expected {expected} parameters, found {found}")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Mysql(err) => Some(err),
            _ => None,
        }
    }
}
