use super::{connector::Sqlite, error::Error, util::sqlite_ref_to_usql};
use rusqlite::types::{FromSql, FromSqlError, Value as SqliteValue, ValueRef as SqliteValueRef};
use std::{borrow::Cow, collections::HashMap, string::String, sync::Arc, vec::Vec};
use usql_core::Connector;
use usql_value::{JsonValue, Type, Value, ValueCow, ValueRef};

pub trait ColumnIndex {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a SqliteValue>;
}

impl ColumnIndex for usize {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a SqliteValue> {
        row.values.get(*self)
    }
}

impl ColumnIndex for &str {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a SqliteValue> {
        row.columns.get(*self).and_then(|m| row.values.get(*m))
    }
}

impl<'b> ColumnIndex for &Cow<'b, str> {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a SqliteValue> {
        row.columns
            .get(self.as_ref())
            .and_then(|m| row.values.get(*m))
    }
}

impl ColumnIndex for usql_core::ColumnIndex<'_> {
    fn get<'a>(&self, row: &'a Row) -> Option<&'a SqliteValue> {
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
    pub fn get_ref<T: ColumnIndex>(&self, name: T) -> Option<SqliteValueRef<'_>> {
        name.get(self).map(|m| m.into())
    }

    pub fn get_raw<T: ColumnIndex>(&self, name: T) -> Option<&SqliteValue> {
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

impl usql_core::Row for Row {
    type Connector = Sqlite;

    fn get<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        self.get_raw(index)
            .map(|m| ValueCow::Ref(sqlite_ref_to_usql(m)))
            .ok_or_else(|| Error::NotFound)
    }

    fn get_typed<'a>(
        &'a self,
        index: usql_core::ColumnIndex<'_>,
        ty: Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        let value = usql_core::Row::get(self, index)?;
        get_typed(value, ty)
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

fn get_typed<'a>(value: ValueCow<'a>, ty: Type) -> Result<ValueCow<'a>, Error> {
    if value.as_ref().is_null() {
        return Ok(value);
    }

    let value = match ty {
        Type::Text => {
            if !value.as_ref().is_text() {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Text,
                });
            }
            value
        }
        Type::SmallInt => match value.as_ref() {
            ValueRef::BigInt(i) => Value::SmallInt(i as _).into(),
            ValueRef::Int(i) => Value::SmallInt(i as _).into(),
            ValueRef::SmallInt(i) => Value::SmallInt(i).into(),
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::SmallInt,
                });
            }
        },
        Type::BigInt => match value.as_ref() {
            ValueRef::BigInt(i) => Value::BigInt(i).into(),
            ValueRef::Int(i) => Value::BigInt(i as _).into(),
            ValueRef::SmallInt(i) => Value::BigInt(i as _).into(),
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::BigInt,
                });
            }
        },
        Type::Int => match value.as_ref() {
            ValueRef::BigInt(i) => Value::Int(i as _).into(),
            ValueRef::Int(i) => Value::Int(i).into(),
            ValueRef::SmallInt(i) => Value::Int(i as _).into(),
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Int,
                });
            }
        },
        Type::Blob => {
            if !value.as_ref().is_binary() {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Blob,
                });
            }
            value
        }
        Type::Time => match value.as_ref() {
            ValueRef::Text(text) => {
                let time = chrono::NaiveTime::parse_from_str(text, "%T%.f").unwrap();
                Value::Time(time).into()
            }
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Time,
                });
            }
        },
        Type::Date => match value.as_ref() {
            ValueRef::Text(text) => {
                let date = chrono::NaiveDate::parse_from_str(text, "%F").unwrap();
                Value::Date(date).into()
            }
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Date,
                });
            }
        },
        Type::DateTime => match value.as_ref() {
            ValueRef::Text(text) => {
                let date = chrono::NaiveDateTime::parse_from_str(text, "%F %T%.f").unwrap();
                Value::Timestamp(date).into()
            }
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::DateTime,
                });
            }
        },
        Type::Json => match value.as_ref() {
            ValueRef::Text(text) => {
                let json: JsonValue = serde_json::from_str(text).unwrap();
                Value::Json(json).into()
            }
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Json,
                });
            }
        },
        Type::Real => match value.as_ref() {
            ValueRef::Float(f) => Value::Double((*f as f64).into()).into(),
            ValueRef::Double(f) => Value::Double(f).into(),
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Real,
                });
            }
        },
        Type::Double => match value.as_ref() {
            ValueRef::Float(f) => Value::Float((*f as f32).into()).into(),
            ValueRef::Double(f) => Value::Float((*f as f32).into()).into(),
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Double,
                });
            }
        },
        Type::Uuid => match value.as_ref() {
            ValueRef::ByteArray(bs) => {
                let id = uuid::Uuid::from_slice(&*bs).unwrap();
                Value::Uuid(id).into()
            }
            _ => {
                return Err(Error::Convert {
                    found: value.as_ref().get_type(),
                    expected: Type::Uuid,
                });
            }
        },
        Type::Bool => {
            let b = match value.as_ref() {
                ValueRef::Bool(b) => return Ok(Value::Bool(b).into()),
                ValueRef::BigInt(b) => b as u8,
                ValueRef::SmallInt(b) => b as _,
                ValueRef::Int(b) => b as _,
                _ => {
                    return Err(Error::Convert {
                        found: value.as_ref().get_type(),
                        expected: Type::Bool,
                    });
                }
            };

            Value::Bool(if b == 0 { false } else { true }).into()
        }
        Type::Array(_item) => {
            //
            match value.as_ref() {
                ValueRef::Array(_a) => {
                    unimplemented!("Array")
                }
                v => {
                    return Err(Error::Convert {
                        found: v.get_type(),
                        expected: Type::Bool,
                    });
                }
            }
        }
        Type::Any => value,
    };

    Ok(value)
}
