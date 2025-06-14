use usql_core::{Row, Value, ValueCow};

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

impl Row for libsql::Row {
    type Connector = LibSql;

    fn get<'a>(
        &'a self,
        index: crate::ColumnIndex<'_>,
    ) -> Result<crate::ValueCow<'a>, <Self::Connector as crate::Connector>::Error> {
        match index {
            crate::ColumnIndex::Named(named) => {
                let Some(idx) = row_index(self, named) else {
                    return Err(super::error::Error::NotFound);
                };

                let value = self.get_value(idx)?;
                Ok(value_ref(value))
            }
            crate::ColumnIndex::Index(idx) => {
                let value = self.get_value(idx as i32)?;
                Ok(value_ref(value))
            }
        }
    }

    fn len(&self) -> usize {
        self.column_count() as usize
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.column_name(idx as i32)
    }
}
