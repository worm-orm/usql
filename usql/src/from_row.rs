use crate::{error::Error, row::Row};
use usql_core::Connector;

pub trait FromRow: Sized {
    fn from_row<B: Connector>(row: Row<B>) -> Result<Self, Error<B>>;
}
