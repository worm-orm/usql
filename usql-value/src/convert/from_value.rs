use alloc::{string::String, vec::Vec};

use super::ValueConversionError;
use crate::Value;
pub trait FromValue: Sized {
    type Error;
    fn from_value(value: Value) -> Result<Self, Self::Error>;
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    type Error = T::Error;
    fn from_value(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(None),
            value => Ok(Some(T::from_value(value)?)),
        }
    }
}

impl FromValue for bool {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for i16 {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::SmallInt(i) = value {
            Ok(i)
        } else {
            Err(Self::Error::NotInt)
        }
    }
}

impl FromValue for i32 {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for i64 {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for f32 {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for f64 {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for String {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for crate::Atom {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for bytes::Bytes {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for chrono::NaiveDate {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for chrono::NaiveTime {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for chrono::NaiveDateTime {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for uuid::Uuid {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl FromValue for crate::JsonValue {
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue<Error = ValueConversionError>,
{
    type Error = ValueConversionError;

    fn from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::Array(arr) = value {
            arr.into_iter()
                .map(T::from_value)
                .collect::<Result<Vec<_>, _>>()
        } else {
            Err(Self::Error::NotJson)
        }
    }
}

#[cfg(feature = "bycat-value")]
mod bycat {
    use alloc::{
        string::{String, ToString},
        sync::Arc,
        vec::Vec,
    };
    use bycat_value::{Number, Value};
    use core::fmt;

    use crate::{JsonValue, convert::FromValue};

    #[derive(Debug)]
    pub struct BycatConvertError {
        message: &'static str,
    }

    impl fmt::Display for BycatConvertError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Could not convert value: {}", self.message)
        }
    }

    impl core::error::Error for BycatConvertError {}

    impl FromValue for Value {
        type Error = BycatConvertError;

        fn from_value(value: crate::Value) -> Result<Self, Self::Error> {
            let ret = match value {
                crate::Value::Null => Value::Null,
                crate::Value::Bool(e) => Value::Bool(e),
                crate::Value::SmallInt(e) => Value::Number(e.into()),
                crate::Value::Int(e) => Value::Number(e.into()),
                crate::Value::BigInt(e) => Value::Number(e.into()),
                crate::Value::Float(ordered_float) => Value::Number((*ordered_float).into()),
                crate::Value::Double(ordered_float) => Value::Number((*ordered_float).into()),
                crate::Value::Text(atom) => {
                    let s: Arc<String> = atom.into();
                    Value::String(s.into())
                }
                crate::Value::ByteArray(bytes) => Value::Bytes(bytes.to_vec().into()),
                crate::Value::Date(naive_date) => Value::Date(naive_date.into()),
                crate::Value::Time(naive_time) => Value::Time(naive_time.into()),
                crate::Value::Timestamp(naive_date_time) => Value::DateTime(naive_date_time.into()),
                crate::Value::Uuid(uuid) => uuid.into(),
                crate::Value::Json(json_value) => json_value.into(),
                crate::Value::Array(values) => {
                    let mut list = bycat_value::List::<Value>::default();

                    for v in values {
                        let ret = Self::from_value(v)?;
                        list.push(ret);
                    }

                    list.into()
                }
            };

            Ok(ret)
        }
    }

    impl TryFrom<crate::Value> for Value {
        type Error = BycatConvertError;
        fn try_from(value: crate::Value) -> Result<Self, Self::Error> {
            Value::from_value(value)
        }
    }

    impl From<JsonValue> for Value {
        fn from(value: JsonValue) -> Self {
            match value {
                JsonValue::Null => Value::Null,
                JsonValue::Bool(b) => Value::Bool(b),
                JsonValue::Float(ordered_float) => Value::Number((*ordered_float).into()),
                JsonValue::Integer(i) => Value::Number(i.into()),
                JsonValue::String(s) => Value::String(s.into()),
                JsonValue::Array(json_values) => {
                    let mut list = bycat_value::List::<Value>::with_capacity(json_values.len());

                    for v in json_values {
                        list.push(v);
                    }

                    Value::List(list)
                }
                JsonValue::Object(btree_map) => {
                    let mut map = bycat_value::Map::default();

                    for (k, v) in btree_map {
                        map.insert(k, v);
                    }

                    Value::Map(map)
                }
            }
        }
    }

    impl From<Value> for JsonValue {
        fn from(value: Value) -> Self {
            match value {
                Value::Bool(e) => JsonValue::Bool(e),
                Value::String(e) => JsonValue::String(e.into()),
                Value::Bytes(bytes) => JsonValue::Array(
                    bytes
                        .iter()
                        .map(|m| JsonValue::Integer(*m as _))
                        .collect::<_>(),
                ),
                Value::List(list) => JsonValue::Array(list.into_iter().map(|m| m.into()).collect()),
                Value::Map(map) => {
                    JsonValue::Object(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
                }
                Value::Number(number) => {
                    if number.is_float() {
                        JsonValue::Float(number.as_f64().into())
                    } else {
                        JsonValue::Integer(number.as_i64())
                    }
                }
                Value::DateTime(date_time) => JsonValue::String(date_time.to_string()),
                Value::Date(date) => JsonValue::String(date.to_string()),
                Value::Time(time) => JsonValue::String(time.to_string()),
                Value::Null => JsonValue::Null,
            }
        }
    }

    impl TryFrom<Value> for crate::Value {
        type Error = BycatConvertError;
        fn try_from(value: Value) -> Result<Self, Self::Error> {
            let ret = match value {
                Value::Bool(b) => crate::Value::Bool(b),
                Value::String(s) => crate::Value::Text(s.as_str().into()),
                Value::Bytes(bytes) => crate::Value::ByteArray(bytes.to_vec().into()),
                Value::List(list) => {
                    let mut out = Vec::with_capacity(list.len());
                    for item in list {
                        out.push(item.try_into()?);
                    }
                    crate::Value::Array(out)
                }
                Value::Map(map) => crate::Value::Json(Value::Map(map).into()),
                Value::Number(number) => {
                    //
                    match number {
                        Number::I8(_) | Number::U8(_) | Number::I16(_) | Number::U16(_) => {
                            crate::Value::SmallInt(number.as_i16())
                        }
                        Number::I32(_) | Number::U32(_) => crate::Value::Int(number.as_i32()),
                        Number::I64(_) | Number::U64(_) => crate::Value::BigInt(number.as_i64()),
                        Number::F32(f) => crate::Value::Float(f.into()),
                        Number::F64(f) => crate::Value::Double(f.into()),
                    }
                }
                Value::DateTime(date_time) => crate::Value::Timestamp(
                    date_time
                        .try_into()
                        .map_err(|message| BycatConvertError { message })?,
                ),
                Value::Date(date) => {
                    let Some(date) = chrono::NaiveDate::from_ymd_opt(
                        date.year() as _,
                        date.month() as _,
                        date.day() as _,
                    ) else {
                        return Err(BycatConvertError {
                            message: "Not a valid date",
                        });
                    };

                    crate::Value::Date(date)
                }
                Value::Time(time) => {
                    let Some(time) = chrono::NaiveTime::from_hms_nano_opt(
                        time.hour(),
                        time.minute(),
                        time.second(),
                        time.nanosecond(),
                    ) else {
                        return Err(BycatConvertError {
                            message: "Not a valid time",
                        });
                    };

                    crate::Value::Time(time)
                }
                Value::Null => crate::Value::Null,
            };

            Ok(ret)
        }
    }
}
