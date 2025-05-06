use crate::Row;

use super::connector::Postgres;

impl Row for tokio_postgres::Row {
    type Connector = Postgres;

    fn get<'a>(
        &'a self,
        index: crate::ColumnIndex<'_>,
    ) -> Result<crate::ValueCow<'a>, <Self::Connector as crate::Connector>::Error> {
        todo!()
    }

    fn len(&self) -> usize {
        todo!()
    }
}
