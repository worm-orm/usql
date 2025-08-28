use crate::{error::Error, typed::Typed};
use alloc::boxed::Box;
use usql_core::{ColumnIndex, Connector, Row as _};
use usql_value::Value;

pub trait IntoColumnIndex<'a> {
    fn into_column_index(self) -> ColumnIndex<'a>;
}

impl<'a> IntoColumnIndex<'a> for &'a str {
    fn into_column_index(self) -> ColumnIndex<'a> {
        ColumnIndex::Named(self)
    }
}

impl<'a> IntoColumnIndex<'a> for usize {
    fn into_column_index(self) -> ColumnIndex<'a> {
        ColumnIndex::Index(self)
    }
}

pub struct Row<B: Connector> {
    pub(crate) row: B::Row,
}

impl<B: Connector> Row<B> {
    pub fn try_get<'a, T, I>(&self, colunm: I) -> Result<T, Error<B>>
    where
        I: IntoColumnIndex<'a>,
        T: TryFrom<Value> + Typed,
        T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        let value = self
            .row
            .get_typed(colunm.into_column_index(), T::TYPE)
            .map_err(Error::connector)?;

        value.to_owned().try_into().map_err(Error::query)
    }

    pub fn into_inner(self) -> B::Row {
        self.row
    }
}
