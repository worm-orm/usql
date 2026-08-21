use deadpool_postgres::{CreatePoolError, PoolError};

#[derive(Debug)]
pub struct Error {}

impl From<CreatePoolError> for Error {
    fn from(_value: CreatePoolError) -> Self {
        Error {}
    }
}

impl From<PoolError> for Error {
    fn from(_value: PoolError) -> Self {
        todo!()
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(_value: tokio_postgres::Error) -> Self {
        todo!()
    }
}
