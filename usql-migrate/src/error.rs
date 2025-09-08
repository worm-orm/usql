use usql_core::Connector;

#[derive(Debug, thiserror::Error)]
pub enum Error<T: Connector> {
    #[error("Connector: {0}")]
    Connector(T::Error),
    #[error("Query: {0}")]
    Query(#[from] usql_builder::Error),
    #[error("Load: {0}")]
    Load(Box<dyn core::error::Error + Send + Sync>),
}

impl<T> Error<T>
where
    T: Connector,
{
    pub fn load<E: Into<Box<dyn core::error::Error + Send + Sync>>>(error: E) -> Error<T> {
        Error::Load(error.into())
    }
}
