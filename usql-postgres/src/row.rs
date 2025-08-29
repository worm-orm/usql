use usql_core::ColumnIndex;

use crate::connector::Postgres;

pub struct Row(pub(crate) tokio_postgres::row::Row);

impl usql_core::Row for Row {
    type Connector = Postgres;

    fn get<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
    ) -> Result<usql_value::ValueCow<'a>, <Self::Connector as usql_core::Connector>::Error> {
        todo!()
    }

    fn get_typed<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
        ty: usql_value::Type,
    ) -> Result<usql_value::ValueCow<'a>, <Self::Connector as usql_core::Connector>::Error> {
        todo!()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.0.columns().get(idx).map(|m| m.name())
    }
}
