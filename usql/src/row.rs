use crate::{error::Error, typed::Typed};
use alloc::boxed::Box;
use usql_core::{ColumnIndex, Connector, Row as _};
use usql_value::convert::FromValue;

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
        T: FromValue + Typed,
        T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        let value = self
            .row
            .get_typed(colunm.into_column_index(), T::TYPE)
            .map_err(Error::connector)?;

        value.to_owned().try_get().map_err(Error::query)
    }

    pub fn get<'a, I: IntoColumnIndex<'a>>(
        &'a self,
        index: I,
    ) -> Result<usql_value::ValueCow<'a>, Error<B>> {
        self.row
            .get(index.into_column_index())
            .map_err(Error::connector)
    }

    pub fn get_typed<'a, I: IntoColumnIndex<'a>>(
        &'a self,
        index: I,
        ty: usql_value::Type,
    ) -> Result<usql_value::ValueCow<'a>, Error<B>> {
        self.row
            .get_typed(index.into_column_index(), ty)
            .map_err(Error::connector)
    }

    pub fn into_inner(self) -> B::Row {
        self.row
    }
}

impl<B: Connector> usql_core::Row for Row<B> {
    type Connector = B;

    fn get<'a>(
        &'a self,
        index: ColumnIndex<'_>,
    ) -> Result<usql_value::ValueCow<'a>, <Self::Connector as Connector>::Error> {
        self.row.get(index)
    }

    fn get_typed<'a>(
        &'a self,
        index: ColumnIndex<'_>,
        ty: usql_value::Type,
    ) -> Result<usql_value::ValueCow<'a>, <Self::Connector as Connector>::Error> {
        self.row.get_typed(index, ty)
    }

    fn len(&self) -> usize {
        self.row.len()
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.row.column_name(idx)
    }
}
