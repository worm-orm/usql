use alloc::boxed::Box;
use core::fmt;
use usql_core::Connector;

pub enum Error<B: Connector> {
    Connector(B::Error),
    Query(Box<dyn core::error::Error + Send + Sync>),
    Unknown(Box<dyn core::error::Error + Send + Sync>),
    NotFound,
}

impl<B: Connector> fmt::Debug for Error<B>
where
    B::Error: core::error::Error,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Connector(e) => write!(f, "{:?}", e),
            Error::Query(err) => write!(f, "{:?}", err),
            Error::Unknown(err) => write!(f, "{:?}", err),
            Error::NotFound => write!(f, "NotFound"),
        }
    }
}

impl<B: Connector> fmt::Display for Error<B>
where
    B::Error: core::error::Error,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Connector(e) => write!(f, "Connector error: {}", e),
            Error::Query(err) => write!(f, "Query error: {}", err),
            Error::Unknown(err) => write!(f, "Unknown: {}", err),
            Error::NotFound => write!(f, "NotFound"),
        }
    }
}

impl<B: Connector> core::error::Error for Error<B>
where
    B::Error: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Connector(e) => Some(e),
            Error::Query(err) => Some(&**err),
            Error::Unknown(err) => Some(&**err),
            Error::NotFound => None,
        }
    }
}

impl<B: Connector> Error<B> {
    pub fn connector(error: B::Error) -> Error<B> {
        Error::Connector(error)
    }

    pub fn query<E>(error: E) -> Error<B>
    where
        E: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        Error::Query(error.into())
    }

    pub fn unknown<E>(error: E) -> Error<B>
    where
        E: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        Error::Unknown(error.into())
    }
}

impl<B: Connector> From<usql_builder::Error> for Error<B> {
    fn from(value: usql_builder::Error) -> Self {
        Error::query(value)
    }
}
