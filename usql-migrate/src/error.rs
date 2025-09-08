use usql_core::Connector;

#[derive(thiserror::Error)]
pub enum Error<T: Connector> {
    #[error("Connector: {0}")]
    Connector(T::Error),
    #[error("Query: {0}")]
    Query(#[from] usql_builder::Error),
    #[error("Load: {0}")]
    Load(Box<dyn core::error::Error + Send + Sync>),
}

impl<T> core::fmt::Debug for Error<T>
where
    T: Connector,
    T::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Connector(err) => f.debug_tuple("Connector").field(err).finish(),
            Error::Query(err) => f.debug_tuple("Query").field(err).finish(),
            Error::Load(err) => f.debug_tuple("Load").field(err).finish(),
        }
    }
}

impl<T> Error<T>
where
    T: Connector,
{
    pub fn load<E: Into<Box<dyn core::error::Error + Send + Sync>>>(error: E) -> Error<T> {
        Error::Load(error.into())
    }
}
