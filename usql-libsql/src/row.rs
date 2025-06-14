use usql_core::{Connector, Value, ValueCow};

use super::connector::LibSql;

fn row_index(row: &libsql::Row, name: &str) -> Option<i32> {
    for idx in 0..row.column_count() {
        let column_name = row.column_name(idx).expect("column name");
        if column_name == name {
            return Some(idx);
        }
    }

    None
}

fn value_ref<'a>(value: libsql::Value) -> ValueCow<'a> {
    match value {
        libsql::Value::Null => ValueCow::Owned(Value::Null),
        libsql::Value::Integer(i) => ValueCow::Owned(Value::BigInt(i)),
        libsql::Value::Real(i) => ValueCow::Owned(Value::Double(i.into())),
        libsql::Value::Text(i) => ValueCow::Owned(Value::Text(i)),
        libsql::Value::Blob(items) => ValueCow::Owned(Value::ByteArray(items.into())),
    }
}

pub struct Row(pub libsql::Row);

impl usql_core::Row for Row {
    type Connector = LibSql;

    fn get<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        match index {
            usql_core::ColumnIndex::Named(named) => {
                let Some(idx) = row_index(&self.0, named) else {
                    return Err(super::error::Error::NotFound);
                };

                let value = self.0.get_value(idx)?;
                Ok(value_ref(value))
            }
            usql_core::ColumnIndex::Index(idx) => {
                let value = self.0.get_value(idx as i32)?;
                Ok(value_ref(value))
            }
        }
    }

    fn get_typed<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
        ty: usql_core::Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        todo!()
    }

    fn len(&self) -> usize {
        self.0.column_count() as usize
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.0.column_name(idx as i32)
    }
}
