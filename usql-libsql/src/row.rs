use crate::Error;

use super::connector::LibSql;
use usql_core::Connector;
use usql_value::{JsonValue, Type, Value, ValueCow, ValueRef, chrono};

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
        ty: Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error> {
        let value = self.get(index)?;
        get_typed(value, ty)
    }

    fn len(&self) -> usize {
        self.0.column_count() as usize
    }

    fn column_name(&self, idx: usize) -> Option<&str> {
        self.0.column_name(idx as i32)
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
                let date = chrono::NaiveDateTime::parse_from_str(text, "%+").unwrap();
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
