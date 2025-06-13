use core::fmt;
use std::fmt::Pointer;

use usql_core::Connector;

pub enum Error<B: Connector> {
    Connector(B::Error),
    Query(Box<dyn core::error::Error + Send + Sync>),
    NotFound,
}

impl<B: Connector> fmt::Debug for Error<B>
where
    B::Error: core::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connector(e) => write!(f, "{:?}", e),
            Error::Query(err) => write!(f, "{:?}", err),
            Error::NotFound => write!(f, "NotFound"),
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
}
