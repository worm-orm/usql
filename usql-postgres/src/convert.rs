use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use tokio_postgres::types::{Kind, Type as PgType};
use usql_value::{JsonValue, Type, Value, geob::Geob};

use crate::error::Error;

/// Convert a value read from a column of type `column`
pub fn convert(value: Value, ty: &Type, column: &PgType) -> Result<Value, Error> {
    if let Kind::Domain(inner) = column.kind() {
        return convert(value, ty, inner);
    }

    let found = value.get_type();
    if value.is_null() {
        return Ok(value);
    }

    if let (Type::Array(item), Value::Array(_), Kind::Array(column)) = (ty, &value, column.kind()) {
        let Value::Array(values) = value else {
            unreachable!()
        };
        return values
            .into_iter()
            .map(|value| convert(value, item, column))
            .collect::<Result<_, _>>()
            .map(Value::Array);
    }

    // Numerics are read as text
    let numeric = *column == PgType::NUMERIC;

    let output = match (ty, value) {
        (Type::Any, value) => Some(value),
        (Type::Text, value @ Value::Text(_)) => Some(value),
        (Type::SmallInt, value) => int(&value, numeric)
            .and_then(|i| i16::try_from(i).ok())
            .map(Value::from),
        (Type::Int, value) => int(&value, numeric)
            .and_then(|i| i32::try_from(i).ok())
            .map(Value::from),
        (Type::BigInt, value) => int(&value, numeric).map(Value::from),
        (Type::Float, Value::Float(f)) => Some(Value::Float(f)),
        (Type::Float, value) => float(&value, numeric).and_then(|f| {
            let narrow = f as f32;
            (narrow.is_finite() || !f.is_finite()).then(|| narrow.into())
        }),
        (Type::Double, value) => float(&value, numeric).map(Value::from),
        (Type::Bool, value @ Value::Bool(_)) => Some(value),
        (Type::Bool, value) => int(&value, false).map(|i| Value::Bool(i != 0)),
        (Type::Blob, value @ Value::ByteArray(_)) => Some(value),
        (Type::Date, value @ Value::Date(_)) => Some(value),
        (Type::Date, Value::Text(text)) => NaiveDate::parse_from_str(text.as_str(), "%F")
            .ok()
            .map(Value::from),
        (Type::Time, value @ Value::Time(_)) => Some(value),
        (Type::Time, Value::Text(text)) => NaiveTime::parse_from_str(text.as_str(), "%T%.f")
            .ok()
            .map(Value::from),
        (Type::DateTime, value @ Value::Timestamp(_)) => Some(value),
        (Type::DateTime, Value::Text(text)) => {
            NaiveDateTime::parse_from_str(text.as_str(), "%F %T%.f")
                .ok()
                .map(Value::from)
        }
        (Type::Uuid, value @ Value::Uuid(_)) => Some(value),
        (Type::Uuid, Value::Text(text)) => {
            uuid::Uuid::parse_str(text.as_str()).ok().map(Value::from)
        }
        (Type::Uuid, Value::ByteArray(bytes)) => {
            uuid::Uuid::from_slice(&bytes).ok().map(Value::from)
        }
        (Type::Json, value @ Value::Json(_)) => Some(value),
        (Type::Json, Value::Text(text)) => serde_json::from_str::<JsonValue>(text.as_str())
            .ok()
            .map(Value::Json),
        (Type::Geometry(geo_ty), value) => {
            let geo = match value {
                Value::Geometry(geo) => Some(geo),
                Value::ByteArray(bytes) => Geob::from_bytes(&*bytes).ok(),
                Value::Text(text) => Geob::from_text(text.as_str()).ok(),
                _ => None,
            };
            match geo {
                Some(geo) if !geo.is(*geo_ty) => {
                    return Err(Error::Convert {
                        found: Some(Type::Geometry(geo.kind())),
                        expected: ty.clone(),
                    });
                }
                geo => geo.map(Value::Geometry),
            }
        }
        _ => None,
    };

    output.ok_or_else(|| Error::Convert {
        found,
        expected: ty.clone(),
    })
}

fn int(value: &Value, numeric: bool) -> Option<i64> {
    match value {
        Value::SmallInt(i) => Some((*i).into()),
        Value::Int(i) => Some((*i).into()),
        Value::BigInt(i) => Some(*i),
        Value::Text(text) if numeric => {
            let text = text.as_str();
            // Only whole numbers, as nothing may be lost
            match text.split_once('.') {
                Some((int, fraction)) if fraction.bytes().all(|b| b == b'0') => int.parse().ok(),
                Some(_) => None,
                None => text.parse().ok(),
            }
        }
        _ => None,
    }
}

fn float(value: &Value, numeric: bool) -> Option<f64> {
    match value {
        Value::Float(f) => Some(f.0.into()),
        Value::Double(f) => Some(f.0),
        Value::Text(text) if numeric => text.as_str().parse().ok(),
        value => int(value, false).map(|i| i as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(value: impl Into<Value>, ty: Type, column: PgType) -> Option<Value> {
        convert(value.into(), &ty, &column).ok()
    }

    #[test]
    fn numbers() {
        assert_eq!(
            run(5i64, Type::SmallInt, PgType::INT8),
            Some(Value::SmallInt(5))
        );
        assert_eq!(run(40_000i64, Type::SmallInt, PgType::INT8), None);
        assert_eq!(
            run("12.00", Type::Int, PgType::NUMERIC),
            Some(Value::Int(12))
        );
        assert_eq!(run("12.50", Type::Int, PgType::NUMERIC), None);
        assert_eq!(
            run("12.50", Type::Double, PgType::NUMERIC),
            Some(12.5f64.into())
        );
        assert_eq!(run("12", Type::Int, PgType::TEXT), None);
        assert_eq!(
            run(1.5f32, Type::Double, PgType::FLOAT4),
            Some(1.5f64.into())
        );
        assert_eq!(
            run(1.5f64, Type::Float, PgType::FLOAT8),
            Some(1.5f32.into())
        );
        assert_eq!(run(f64::MAX, Type::Float, PgType::FLOAT8), None);
    }

    #[test]
    fn invalid_text_is_an_error() {
        for ty in [
            Type::Date,
            Type::Time,
            Type::DateTime,
            Type::Uuid,
            Type::Json,
        ] {
            assert_eq!(run("?", ty, PgType::TEXT), None);
        }
    }

    #[test]
    fn arrays() {
        let value = Value::Array(vec![Value::Int(1), Value::Null]);
        assert_eq!(
            convert(
                value.clone(),
                &Type::Array(Type::BigInt.into()),
                &PgType::INT4_ARRAY
            )
            .unwrap(),
            Value::Array(vec![Value::BigInt(1), Value::Null])
        );
        assert!(convert(value, &Type::Array(Type::Text.into()), &PgType::INT4_ARRAY).is_err());
    }
}
