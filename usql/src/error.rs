use core::fmt;
use std::fmt::Pointer;

use usql_core::Connector;

pub enum Error<B: Connector> {
    Connector(B::Error),
    NotFound,
}

impl<B: Connector> fmt::Debug for Error<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connector(e) => e.fmt(f),
            Error::NotFound => write!(f, "NotFound"),
        }
    }
}

impl<B: Connector> Error<B> {
    pub fn connector(error: B::Error) -> Error<B> {
        Error::Connector(error)
    }

    pub fn query<E>(error: E) -> Error<B> {
        todo!()
    }
}
