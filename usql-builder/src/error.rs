use alloc::fmt;

#[derive(Debug)]
pub enum Error {}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        todo!()
    }
}
