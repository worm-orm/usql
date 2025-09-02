use core::fmt;

use usql::core::Connector;

pub enum Error<T: Connector> {
    Connector(T::Error),
    Builder(usql::builder::Error),
    Unknown(Box<dyn core::error::Error + Send + Sync>),
}

impl<T> fmt::Debug for Error<T>
where
    T: Connector,
    T::Error: core::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Builder(err) => f.debug_tuple("Builder").field(err).finish(),
            Error::Unknown(err) => f.debug_tuple("Unknown").field(err).finish(),
            Error::Connector(err) => f.debug_tuple("Connector").field(err).finish(),
        }
    }
}

impl<T: Connector> std::fmt::Display for Error<T>
where
    T::Error: core::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Builder(err) => write!(f, "Builder error: {}", err),
            Error::Unknown(err) => write!(f, "Unknown error: {}", err),
            Error::Connector(err) => write!(f, "Connector error: {}", err),
        }
    }
}

impl<T: Connector> std::error::Error for Error<T>
where
    T::Error: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Builder(err) => Some(err),
            Error::Unknown(err) => Some(&**err),
            Error::Connector(err) => Some(&*err),
        }
    }
}

impl<T: Connector> Error<T> {
    pub(crate) fn conn(error: T::Error) -> Error<T> {
        Error::Connector(error)
    }

    pub fn new<E: Into<Box<dyn core::error::Error + Send + Sync>>>(error: E) -> Error<T> {
        Error::Unknown(error.into())
    }
}

impl<T: Connector> From<usql::builder::Error> for Error<T> {
    fn from(value: usql::builder::Error) -> Self {
        Error::Builder(value)
    }
}
