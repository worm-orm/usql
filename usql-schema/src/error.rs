use usql_core::{Connector, System};

#[derive(thiserror::Error)]
pub enum Error<T: Connector> {
    #[error("Connector: {0}")]
    Connector(T::Error),
    #[error("Unexpected value in column '{0}'")]
    Value(&'static str),
    #[error("Unsupported database: {0:?}")]
    Unsupported(System),
    #[error("No current schema")]
    NoSchema,
}

impl<T> core::fmt::Debug for Error<T>
where
    T: Connector,
    T::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Connector(err) => f.debug_tuple("Connector").field(err).finish(),
            Error::Value(name) => f.debug_tuple("Value").field(name).finish(),
            Error::Unsupported(system) => f.debug_tuple("Unsupported").field(system).finish(),
            Error::NoSchema => f.write_str("NoSchema"),
        }
    }
}
