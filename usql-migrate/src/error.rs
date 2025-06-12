#[derive(Debug)]
pub struct Error {}

impl Error {
    pub fn new<E: Into<Box<dyn core::error::Error + Send + Sync>>>(error: E) -> Error {
        Error {}
    }
}

impl From<usql_builder::Error> for Error {
    fn from(value: usql_builder::Error) -> Self {
        todo!()
    }
}
