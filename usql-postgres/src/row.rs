use usql_core::{ColumnIndex, Connector};
use usql_value::{Type, Value, ValueCow};

use crate::{connector::Postgres, convert::convert, error::Error};

pub struct Row(pub(crate) tokio_postgres::row::Row);

impl Row {
    fn read(&self, index: ColumnIndex<'_>) -> Result<(Value, usize), Error> {
        let columns = self.0.columns();
        let index = match index {
            ColumnIndex::Index(index) if index < columns.len() => index,
            ColumnIndex::Index(_) => return Err(Error::NotFound),
            ColumnIndex::Named(name) => columns
                .iter()
                .position(|m| m.name() == name)
                .ok_or(Error::NotFound)?,
        };

        Ok((self.0.try_get(index)?, index))
    }
}

impl usql_core::Row for Row {
    type Connector = Postgres;

    fn get<'a>(
        &'a self,
        index: ColumnIndex<'_>,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        Ok(self.read(index)?.0.into())
    }

    fn get_typed<'a>(
        &'a self,
        index: ColumnIndex<'_>,
        ty: Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        let (value, index) = self.read(index)?;
        let column = self.0.columns()[index].type_();
        Ok(convert(value, &ty, column)?.into())
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.0.columns().get(idx).map(|m| m.name())
    }
}
