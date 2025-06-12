use rusqlite::types::{FromSql, FromSqlError, ValueRef as SqliteValue};
use std::{collections::HashMap, string::String, sync::Arc, vec::Vec};

use crate::{Value, ValueCow, ValueRef};

use super::{connector::Sqlite, error::Error, util::sqlite_ref_to_usql};

pub trait ColumnIndex {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a Value>;
}

impl ColumnIndex for usize {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a Value> {
        row.values.get(*self)
    }
}

impl ColumnIndex for &str {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a Value> {
        row.columns.get(*self).and_then(|m| row.values.get(*m))
    }
}

impl ColumnIndex for crate::ColumnIndex<'_> {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a Value> {
        match self {
            Self::Index(idx) => idx.get(row),
            Self::Named(name) => name.get(row),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub(crate) columns: Arc<HashMap<String, usize>>,
    pub(crate) values: Vec<rusqlite::types::Value>,
}

impl Row {
    pub fn get_ref<T: ColumnIndex>(&self, name: T) -> Option<SqliteValue<'_>> {
        name.get(self).map(|m| m.into())
    }

    pub fn get_raw<T: ColumnIndex>(&self, name: T) -> Option<&Value> {
        name.get(self)
    }

    pub fn get<T: FromSql, I: ColumnIndex>(
        &self,
        column: I,
    ) -> Result<T, rusqlite::types::FromSqlError> {
        let Some(value) = self.get_ref(column) else {
            return Err(FromSqlError::Other("field not found".into()));
        };

        T::column_result(value)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn column_names(&self) -> std::collections::hash_map::Keys<'_, String, usize> {
        self.columns.keys()
    }

    pub fn values(&self) -> &[rusqlite::types::Value] {
        &self.values
    }

    pub fn into_values(self) -> Vec<rusqlite::types::Value> {
        self.values
    }
}

impl crate::Row for Row {
    type Connector = Sqlite;

    fn get<'a>(
        &'a self,
        index: crate::ColumnIndex<'_>,
    ) -> Result<crate::ValueCow<'a>, <Self::Connector as crate::Connector>::Error> {
        self.get_raw(index)
            .map(|m| ValueCow::Ref(sqlite_ref_to_usql(m)))
            .ok_or_else(|| Error::NotFound)
    }

    fn get_typed<'a>(
        &'a self,
        index: crate::ColumnIndex<'_>,
        ty: crate::Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as crate::Connector>::Error> {
        let value = crate::Row::get(self, index)?;

        let value = match ty {
            crate::Type::Text => {
                if !value.as_ref().is_text() {
                    panic!("type error")
                }
                value
            }
            crate::Type::SmallInt => {
                //
                match value.as_ref() {
                    ValueRef::BigInt(i) => Value::SmallInt(i as _).into(),
                    ValueRef::Int(i) => Value::SmallInt(i as _).into(),
                    ValueRef::SmallInt(i) => Value::SmallInt(i).into(),
                    _ => {
                        panic!("type error")
                    }
                }
            }
            crate::Type::BigInt => match value.as_ref() {
                ValueRef::BigInt(i) => Value::BigInt(i).into(),
                ValueRef::Int(i) => Value::BigInt(i as _).into(),
                ValueRef::SmallInt(i) => Value::BigInt(i).into(),
                _ => {
                    panic!("type error")
                }
            },
            crate::Type::Int => match value.as_ref() {
                ValueRef::BigInt(i) => Value::Int(i as _).into(),
                ValueRef::Int(i) => Value::Int(i).into(),
                ValueRef::SmallInt(i) => Value::Int(i as _).into(),
                _ => {
                    panic!("type error")
                }
            },
            crate::Type::Blob => {
                todo!()
            }
            crate::Type::Time => {
                todo!()
            }
            crate::Type::Date => todo!(),
            crate::Type::DateTime => todo!(),
            crate::Type::Json => todo!(),
            crate::Type::Real => todo!(),
            crate::Type::Double => todo!(),
            crate::Type::Uuid => todo!(),
            crate::Type::Bool => todo!(),
            crate::Type::Array(_) => todo!(),
            crate::Type::Any => value,
        };

        Ok(value)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        for (k, v) in &*self.columns {
            if v == &idx {
                return Some(k);
            }
        }

        None
    }
}
