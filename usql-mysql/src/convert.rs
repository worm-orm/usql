use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use mysql_async::{
    Column, Value as MyValue,
    consts::{ColumnFlags, ColumnType},
};
use usql_value::{JsonValue, Type, Value, ValueRef};

use crate::error::Error;

/// The character set of binary strings and blobs
const BINARY_CHARSET: u16 = 63;

/// Convert a parameter to the value sent to the server
pub fn to_param(value: ValueRef<'_>) -> Result<MyValue, Error> {
    Ok(match value {
        ValueRef::Null => MyValue::NULL,
        ValueRef::Bool(b) => MyValue::Int(b.into()),
        ValueRef::SmallInt(i) => MyValue::Int(i.into()),
        ValueRef::Int(i) => MyValue::Int(i.into()),
        ValueRef::BigInt(i) => MyValue::Int(i),
        ValueRef::Float(f) => MyValue::Float(f.0),
        ValueRef::Double(f) => MyValue::Double(f.0),
        ValueRef::Text(text) => MyValue::Bytes(text.as_bytes().to_vec()),
        ValueRef::ByteArray(bytes) => MyValue::Bytes(bytes.to_vec()),
        ValueRef::Date(date) => MyValue::Date(
            year(date.year())?,
            date.month() as u8,
            date.day() as u8,
            0,
            0,
            0,
            0,
        ),
        ValueRef::Time(time) => MyValue::Time(
            false,
            0,
            time.hour() as u8,
            time.minute() as u8,
            time.second() as u8,
            micros(time.nanosecond()),
        ),
        ValueRef::Timestamp(ts) => MyValue::Date(
            year(ts.year())?,
            ts.month() as u8,
            ts.day() as u8,
            ts.hour() as u8,
            ts.minute() as u8,
            ts.second() as u8,
            micros(ts.nanosecond()),
        ),
        ValueRef::Uuid(uuid) => MyValue::Bytes(uuid.as_bytes().to_vec()),
        ValueRef::Json(json) => {
            MyValue::Bytes(serde_json::to_vec(json).map_err(|_| Error::Unsupported("json value"))?)
        }
        ValueRef::Array(_) => return Err(Error::Unsupported("arrays")),
        ValueRef::Geometry(_) => return Err(Error::Unsupported("geometry")),
    })
}

fn year(year: i32) -> Result<u16, Error> {
    // MySQL supports the years 1000 to 9999
    u16::try_from(year)
        .ok()
        .filter(|y| *y <= 9999)
        .ok_or(Error::Unsupported("years outside 0-9999"))
}

fn micros(nanos: u32) -> u32 {
    // Leap seconds are represented with nanoseconds above a second
    (nanos % 1_000_000_000) / 1000
}

fn is_binary(column: &Column) -> bool {
    column.character_set() == BINARY_CHARSET
}

fn is_decimal(ty: ColumnType) -> bool {
    matches!(
        ty,
        ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL
    )
}

/// Decode a value read from `column`
pub fn decode(value: &MyValue, column: &Column) -> Result<Value, Error> {
    let ty = column.column_type();
    let unsigned = column.flags().contains(ColumnFlags::UNSIGNED_FLAG);

    let output = match value {
        MyValue::NULL => Some(Value::Null),
        MyValue::Int(i) => Some(int(*i as i128, ty, unsigned, column.column_length())),
        MyValue::UInt(i) => Some(int(*i as i128, ty, unsigned, column.column_length())),
        MyValue::Float(f) => Some(Value::from(*f)),
        MyValue::Double(f) => Some(Value::from(*f)),
        MyValue::Date(y, m, d, h, mi, s, us) => {
            let date = NaiveDate::from_ymd_opt((*y).into(), (*m).into(), (*d).into());
            match ty {
                ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => {
                    date.map(Value::Date)
                }
                _ => date
                    .and_then(|date| {
                        date.and_hms_micro_opt((*h).into(), (*mi).into(), (*s).into(), *us)
                    })
                    .map(Value::Timestamp),
            }
        }
        MyValue::Time(neg, days, h, mi, s, us) => {
            match (neg, days) {
                (false, 0) => {
                    NaiveTime::from_hms_micro_opt((*h).into(), (*mi).into(), (*s).into(), *us)
                        .map(Value::Time)
                }
                // A duration, which doesn't fit a time of day
                _ => {
                    let sign = if *neg { "-" } else { "" };
                    let hours = u64::from(*days) * 24 + u64::from(*h);
                    Some(Value::from(format!(
                        "{sign}{hours:02}:{mi:02}:{s:02}.{us:06}"
                    )))
                }
            }
        }
        MyValue::Bytes(bytes) => Some(match ty {
            ColumnType::MYSQL_TYPE_JSON => serde_json::from_slice::<JsonValue>(bytes)
                .map(Value::Json)
                .map_err(|_| Error::Convert {
                    found: Some(Type::Blob),
                    expected: Type::Json,
                })?,
            ColumnType::MYSQL_TYPE_BIT | ColumnType::MYSQL_TYPE_GEOMETRY => {
                Value::from(bytes.as_slice())
            }
            // Decimals have the binary character set, and are read as text
            _ if is_binary(column) && !is_decimal(ty) => Value::from(bytes.as_slice()),
            _ => match core::str::from_utf8(bytes) {
                Ok(text) => Value::from(text),
                Err(_) => Value::from(bytes.as_slice()),
            },
        }),
    };

    output.ok_or(Error::Decode("invalid date or time"))
}

fn int(i: i128, ty: ColumnType, unsigned: bool, length: u32) -> Value {
    let value = match (ty, unsigned) {
        // BOOLEAN is an alias of TINYINT(1)
        (ColumnType::MYSQL_TYPE_TINY, false) if length == 1 && (i == 0 || i == 1) => {
            return Value::Bool(i == 1);
        }
        (ColumnType::MYSQL_TYPE_TINY, _)
        | (ColumnType::MYSQL_TYPE_SHORT, false)
        | (ColumnType::MYSQL_TYPE_YEAR, _) => i16::try_from(i).ok().map(Value::SmallInt),
        (ColumnType::MYSQL_TYPE_SHORT, true)
        | (ColumnType::MYSQL_TYPE_INT24, _)
        | (ColumnType::MYSQL_TYPE_LONG, false) => i32::try_from(i).ok().map(Value::Int),
        _ => None,
    };

    // The rest, and anything out of range for the column type
    value.unwrap_or_else(|| match i64::try_from(i) {
        Ok(i) => Value::BigInt(i),
        // BIGINT UNSIGNED above i64::MAX
        Err(_) => Value::from(i.to_string()),
    })
}

/// Convert a value read from `column` to `ty`
pub fn convert(value: Value, ty: &Type, column: &Column) -> Result<Value, Error> {
    let found = value.get_type();
    if value.is_null() {
        return Ok(value);
    }

    // Decimals are read as text
    let numeric = is_decimal(column.column_type());

    let output = match (ty, value) {
        (Type::Any, value) => Some(value),
        (Type::Text, value @ Value::Text(_)) => Some(value),
        (Type::SmallInt, value) => int_of(&value, numeric)
            .and_then(|i| i16::try_from(i).ok())
            .map(Value::from),
        (Type::Int, value) => int_of(&value, numeric)
            .and_then(|i| i32::try_from(i).ok())
            .map(Value::from),
        (Type::BigInt, value) => int_of(&value, numeric).map(Value::from),
        (Type::Float, Value::Float(f)) => Some(Value::Float(f)),
        (Type::Float, value) => float_of(&value, numeric).and_then(|f| {
            let narrow = f as f32;
            (narrow.is_finite() || !f.is_finite()).then(|| narrow.into())
        }),
        (Type::Double, value) => float_of(&value, numeric).map(Value::from),
        (Type::Bool, value @ Value::Bool(_)) => Some(value),
        (Type::Bool, value) => int_of(&value, false).map(|i| Value::Bool(i != 0)),
        (Type::Blob, value @ Value::ByteArray(_)) => Some(value),
        (Type::Blob, Value::Text(text)) => Some(Value::from(text.as_str().as_bytes())),
        (Type::Date, value @ Value::Date(_)) => Some(value),
        (Type::Date, Value::Timestamp(ts)) if ts.time() == NaiveTime::MIN => {
            Some(Value::Date(ts.date()))
        }
        (Type::Date, Value::Text(text)) => NaiveDate::parse_from_str(text.as_str(), "%F")
            .ok()
            .map(Value::from),
        (Type::Time, value @ Value::Time(_)) => Some(value),
        (Type::Time, Value::Text(text)) => NaiveTime::parse_from_str(text.as_str(), "%T%.f")
            .ok()
            .map(Value::from),
        (Type::DateTime, value @ Value::Timestamp(_)) => Some(value),
        (Type::DateTime, Value::Date(date)) => {
            Some(Value::Timestamp(date.and_time(NaiveTime::MIN)))
        }
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
        _ => None,
    };

    output.ok_or_else(|| Error::Convert {
        found,
        expected: ty.clone(),
    })
}

fn int_of(value: &Value, numeric: bool) -> Option<i64> {
    match value {
        Value::Bool(b) => Some((*b).into()),
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

fn float_of(value: &Value, numeric: bool) -> Option<f64> {
    match value {
        Value::Float(f) => Some(f.0.into()),
        Value::Double(f) => Some(f.0),
        Value::Text(text) if numeric => text.as_str().parse().ok(),
        value => int_of(value, false).map(|i| i as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(ty: ColumnType) -> Column {
        Column::new(ty).with_character_set(33)
    }

    #[test]
    fn params() {
        assert_eq!(to_param(ValueRef::Bool(true)).unwrap(), MyValue::Int(1));
        assert_eq!(
            to_param(ValueRef::Text("hi")).unwrap(),
            MyValue::Bytes(b"hi".to_vec())
        );
        let ts = NaiveDate::from_ymd_opt(2024, 2, 3)
            .unwrap()
            .and_hms_micro_opt(4, 5, 6, 7)
            .unwrap();
        assert_eq!(
            to_param(ValueRef::Timestamp(ts)).unwrap(),
            MyValue::Date(2024, 2, 3, 4, 5, 6, 7)
        );
        let uuid = uuid::Uuid::new_v4();
        assert_eq!(
            to_param(ValueRef::Uuid(uuid)).unwrap(),
            MyValue::Bytes(uuid.as_bytes().to_vec())
        );
        let json = JsonValue::String("a \"quoted\" string".into());
        assert_eq!(
            to_param(ValueRef::Json(&json)).unwrap(),
            MyValue::Bytes(br#""a \"quoted\" string""#.to_vec())
        );
        assert!(to_param(ValueRef::Array(&[])).is_err());
    }

    #[test]
    fn decode_values() {
        let boolean = column(ColumnType::MYSQL_TYPE_TINY).with_column_length(1);
        assert_eq!(
            decode(&MyValue::Int(1), &boolean).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            decode(&MyValue::Int(1), &column(ColumnType::MYSQL_TYPE_LONG)).unwrap(),
            Value::Int(1)
        );
        let unsigned =
            column(ColumnType::MYSQL_TYPE_LONGLONG).with_flags(ColumnFlags::UNSIGNED_FLAG);
        assert_eq!(
            decode(&MyValue::UInt(u64::MAX), &unsigned).unwrap(),
            Value::from(u64::MAX.to_string())
        );
        assert_eq!(
            decode(
                &MyValue::Date(2024, 2, 3, 0, 0, 0, 0),
                &column(ColumnType::MYSQL_TYPE_DATE)
            )
            .unwrap(),
            Value::Date(NaiveDate::from_ymd_opt(2024, 2, 3).unwrap())
        );
        assert!(
            decode(
                &MyValue::Date(0, 0, 0, 0, 0, 0, 0),
                &column(ColumnType::MYSQL_TYPE_DATETIME)
            )
            .is_err()
        );
        assert_eq!(
            decode(
                &MyValue::Time(true, 1, 2, 3, 4, 5),
                &column(ColumnType::MYSQL_TYPE_TIME)
            )
            .unwrap(),
            Value::from("-26:03:04.000005")
        );
        let blob = Column::new(ColumnType::MYSQL_TYPE_BLOB).with_character_set(BINARY_CHARSET);
        assert_eq!(
            decode(&MyValue::Bytes(b"hi".to_vec()), &blob).unwrap(),
            Value::from(&b"hi"[..])
        );
        assert_eq!(
            decode(
                &MyValue::Bytes(b"hi".to_vec()),
                &column(ColumnType::MYSQL_TYPE_VAR_STRING)
            )
            .unwrap(),
            Value::from("hi")
        );
        assert_eq!(
            decode(
                &MyValue::Bytes(br#"{"a":1}"#.to_vec()),
                &column(ColumnType::MYSQL_TYPE_JSON)
            )
            .unwrap()
            .get_type(),
            Some(Type::Json)
        );
    }

    #[test]
    fn typed() {
        let decimal =
            Column::new(ColumnType::MYSQL_TYPE_NEWDECIMAL).with_character_set(BINARY_CHARSET);
        assert_eq!(
            decode(&MyValue::Bytes(b"12.00".to_vec()), &decimal).unwrap(),
            Value::from("12.00")
        );
        assert_eq!(
            convert(Value::from("12.00"), &Type::Int, &decimal).unwrap(),
            Value::Int(12)
        );
        assert!(convert(Value::from("12.50"), &Type::Int, &decimal).is_err());
        assert_eq!(
            convert(Value::from("12.50"), &Type::Double, &decimal).unwrap(),
            Value::from(12.5f64)
        );
        let text = column(ColumnType::MYSQL_TYPE_VAR_STRING);
        assert!(convert(Value::from("12"), &Type::Int, &text).is_err());
        let uuid = uuid::Uuid::new_v4();
        let binary = Column::new(ColumnType::MYSQL_TYPE_STRING).with_character_set(BINARY_CHARSET);
        assert_eq!(
            convert(Value::from(&uuid.as_bytes()[..]), &Type::Uuid, &binary).unwrap(),
            Value::Uuid(uuid)
        );
        assert_eq!(
            convert(
                Value::SmallInt(1),
                &Type::Bool,
                &column(ColumnType::MYSQL_TYPE_TINY)
            )
            .unwrap(),
            Value::Bool(true)
        );
        for ty in [
            Type::Date,
            Type::Time,
            Type::DateTime,
            Type::Uuid,
            Type::Json,
        ] {
            assert!(convert(Value::from("?"), &ty, &text).is_err());
        }
    }
}
