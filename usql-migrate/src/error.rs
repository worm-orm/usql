#[derive(Debug)]
pub struct Error {}

impl From<usql_builder::Error> for Error {
    fn from(value: usql_builder::Error) -> Self {
        todo!()
    }
}
