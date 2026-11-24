use alloc::{format, vec::Vec};
use bytes::Bytes;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use geob::Geob;
use postgres_types::{FromSql, Json, Kind, Type};

use super::{BoxError, numeric};
use crate::{JsonValue, Value};

impl<'a> FromSql<'a> for Value {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, BoxError> {
        let value = match *ty {
            Type::BOOL => bool::from_sql(ty, raw)?.into(),
            Type::CHAR => i8::from_sql(ty, raw)?.into(),
            Type::INT2 => i16::from_sql(ty, raw)?.into(),
            Type::INT4 => i32::from_sql(ty, raw)?.into(),
            Type::INT8 => i64::from_sql(ty, raw)?.into(),
            Type::OID => i64::from(u32::from_sql(ty, raw)?).into(),
            Type::FLOAT4 => f32::from_sql(ty, raw)?.into(),
            Type::FLOAT8 => f64::from_sql(ty, raw)?.into(),
            // There is no decimal value, and text is exact
            Type::NUMERIC => numeric::decode(raw)?.into(),
            Type::BYTEA => Value::ByteArray(Bytes::copy_from_slice(raw)),
            Type::DATE => NaiveDate::from_sql(ty, raw)?.into(),
            Type::TIME => NaiveTime::from_sql(ty, raw)?.into(),
            Type::TIMESTAMP => NaiveDateTime::from_sql(ty, raw)?.into(),
            Type::TIMESTAMPTZ => DateTime::<Utc>::from_sql(ty, raw)?.naive_utc().into(),
            Type::UUID => uuid::Uuid::from_sql(ty, raw)?.into(),
            Type::JSON | Type::JSONB => {
                let Json(json) = Json::<JsonValue>::from_sql(ty, raw)?;
                Value::Json(json)
            }
            Type::XML => core::str::from_utf8(raw)?.into(),
            Type::VOID => Value::Null,
            _ => match ty.kind() {
                Kind::Domain(inner) => return Value::from_sql(inner, raw),
                Kind::Enum(_) => core::str::from_utf8(raw)?.into(),
                Kind::Array(_) => Value::Array(Vec::<Value>::from_sql(ty, raw)?),
                _ if <&str as FromSql>::accepts(ty) => <&str>::from_sql(ty, raw)?.into(),
                _ if <Geob as FromSql>::accepts(ty) => Value::Geometry(Geob::from_sql(ty, raw)?),
                _ => return Err(format!("unsupported postgres type {ty}").into()),
            },
        };

        Ok(value)
    }

    fn from_sql_null(_ty: &Type) -> Result<Self, BoxError> {
        Ok(Value::Null)
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }
}
