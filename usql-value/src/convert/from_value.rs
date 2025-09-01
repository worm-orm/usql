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
