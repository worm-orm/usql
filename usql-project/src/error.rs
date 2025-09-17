use core::fmt;

use usql_core::Connector;

pub type BoxError = Box<dyn core::error::Error + Send + Sync>;

#[derive(thiserror::Error)]
pub enum Error<T: Connector> {
    #[error("Writer error {0}")]
    Unpack(#[from] UnpackError),
    #[error("Connector error: {0}")]
    Connector(T::Error),
}

impl<T: Connector> fmt::Debug for Error<T>
where
    T::Error: core::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unpack(err) => f.debug_tuple("Writer").field(err).finish(),
            Error::Connector(err) => f.debug_tuple("Connector").field(err).finish(),
        }
    }
}

#[derive(Debug)]
pub struct UnpackError {
    inner: BoxError,
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::error::Error for UnpackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl UnpackError {
    pub fn new<E: Into<BoxError>>(error: E) -> UnpackError {
        UnpackError {
            inner: error.into(),
        }
    }
}
