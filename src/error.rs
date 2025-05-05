use core::fmt;

#[derive(Debug)]
pub struct Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl core::error::Error for Error {}
