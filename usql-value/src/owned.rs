use super::{JsonValue, Type, ValueRef};
use crate::{Atom, convert::FromValue};
use alloc::{string::String, vec::Vec};
use bytes::Bytes;
use geob::Geob;
use ordered_float::OrderedFloat;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(OrderedFloat<f32>),
    Double(OrderedFloat<f64>),
    Text(Atom),
    ByteArray(Bytes),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Timestamp(chrono::NaiveDateTime),
    Uuid(uuid::Uuid),
    Json(JsonValue),
    Array(Vec<Value>),
    Geometry(Geob),
}

impl Value {
    pub fn get_type(&self) -> Option<Type> {
        self.as_ref().get_type()
    }

    pub fn as_ref(&self) -> ValueRef<'_> {
        self.into()
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn try_get<T: FromValue>(self) -> Result<T, T::Error> {
        T::from_value(self)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::SmallInt(value as _)
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::SmallInt(value as _)
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::SmallInt(value)
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value::SmallInt(value as _)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Int(value as _)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::BigInt(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::BigInt(value as _)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(OrderedFloat(value))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Double(OrderedFloat(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Text(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Text(value.into())
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Value::ByteArray(value.to_vec().into())
    }
}

impl From<Bytes> for Value {
    fn from(value: Bytes) -> Self {
        Value::ByteArray(value)
    }
}

impl From<chrono::NaiveDate> for Value {
    fn from(value: chrono::NaiveDate) -> Self {
        Value::Date(value)
    }
}

impl From<chrono::NaiveTime> for Value {
    fn from(value: chrono::NaiveTime) -> Self {
        Value::Time(value)
    }
}

impl From<chrono::NaiveDateTime> for Value {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Value::Timestamp(value)
    }
}

impl From<uuid::Uuid> for Value {
    fn from(value: uuid::Uuid) -> Self {
        Value::Uuid(value)
    }
}

impl From<JsonValue> for Value {
    fn from(value: JsonValue) -> Self {
        Value::Json(value)
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}

macro_rules! geo_impl {
    ($($ty: ty),+) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Value {
                    Value::Geometry(Geob::from_geo_type(&value, geob::SRID::UNKNOWN))
                }
            }
        )*
    };
}

geo_impl!(
    geo_types::Point<f64>,
    geo_types::MultiPoint<f64>,
    geo_types::LineString<f64>,
    geo_types::MultiLineString<f64>,
    geo_types::Polygon<f64>,
    geo_types::MultiPolygon<f64>,
    geo_types::Geometry<f64>
);

impl From<Geob> for Value {
    fn from(value: Geob) -> Self {
        Value::Geometry(value)
    }
}
