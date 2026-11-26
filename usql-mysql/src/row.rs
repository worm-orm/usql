use usql_core::{ColumnIndex, Connector};
use usql_value::{Type, Value, ValueCow};

use crate::{
    connector::Mysql,
    convert::{convert, decode},
    error::Error,
};

pub struct Row(pub(crate) mysql_async::Row);

impl Row {
    fn read(&self, index: ColumnIndex<'_>) -> Result<(Value, usize), Error> {
        let columns = self.0.columns_ref();
        let index = match index {
            ColumnIndex::Index(index) if index < columns.len() => index,
            ColumnIndex::Index(_) => return Err(Error::NotFound),
            ColumnIndex::Named(name) => columns
                .iter()
                .position(|m| m.name_ref() == name.as_bytes())
                .ok_or(Error::NotFound)?,
        };

        let value = self.0.as_ref(index).ok_or(Error::NotFound)?;
        Ok((decode(value, &columns[index])?, index))
    }
}

impl usql_core::Row for Row {
    type Connector = Mysql;

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
        let column = &self.0.columns_ref()[index];
        Ok(convert(value, &ty, column)?.into())
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        let column = self.0.columns_ref().get(idx)?;
        core::str::from_utf8(column.name_ref()).ok()
    }
}
