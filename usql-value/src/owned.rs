use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bytes::Bytes;
use core::convert::TryFrom;
use core::hash::Hash;
use ordered_float::OrderedFloat;

use super::{JsonValue, Type, ValueRef};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Null,
    Bool(bool),
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(OrderedFloat<f32>),
    Double(OrderedFloat<f64>),
    Text(String),
    ByteArray(Bytes),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Timestamp(chrono::NaiveDateTime),
    Uuid(uuid::Uuid),
    Json(JsonValue),
    Array(Vec<Value>),
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
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::SmallInt(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::BigInt(value)
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
        Value::Text(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Text(value.to_string())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueConversionError {
    NotBool,
    NotInt,
    NotBigInt,
    NotFloat,
    NotDouble,
    NotText,
    NotByteArray,
    NotDate,
    NotTime,
    NotTimestamp,
    NotUuid,
    NotJson,
}

impl TryFrom<Value> for bool {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Bool(b) = value {
            Ok(b)
        } else {
            Err(ValueConversionError::NotBool)
        }
    }
}

impl TryFrom<Value> for i32 {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(i) = value {
            Ok(i)
        } else {
            Err(ValueConversionError::NotInt)
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::BigInt(i) = value {
            Ok(i)
        } else {
            Err(ValueConversionError::NotBigInt)
        }
    }
}

impl TryFrom<Value> for f32 {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Float(f) = value {
            Ok(f.into_inner())
        } else {
            Err(ValueConversionError::NotFloat)
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Double(d) = value {
            Ok(d.into_inner())
        } else {
            Err(ValueConversionError::NotDouble)
        }
    }
}

impl TryFrom<Value> for String {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Text(s) = value {
            Ok(s)
        } else {
            Err(ValueConversionError::NotText)
        }
    }
}

impl TryFrom<Value> for Bytes {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::ByteArray(b) = value {
            Ok(b)
        } else {
            Err(ValueConversionError::NotByteArray)
        }
    }
}

impl TryFrom<Value> for chrono::NaiveDate {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Date(d) = value {
            Ok(d)
        } else {
            Err(ValueConversionError::NotDate)
        }
    }
}

impl TryFrom<Value> for chrono::NaiveTime {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Time(t) = value {
            Ok(t)
        } else {
            Err(ValueConversionError::NotTime)
        }
    }
}

impl TryFrom<Value> for chrono::NaiveDateTime {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Timestamp(ts) = value {
            Ok(ts)
        } else {
            Err(ValueConversionError::NotTimestamp)
        }
    }
}

impl TryFrom<Value> for uuid::Uuid {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uuid(u) = value {
            Ok(u)
        } else {
            Err(ValueConversionError::NotUuid)
        }
    }
}

impl TryFrom<Value> for JsonValue {
    type Error = ValueConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Json(v) = value {
            Ok(v)
        } else {
            Err(ValueConversionError::NotJson)
        }
    }
}

impl core::fmt::Display for ValueConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueConversionError::NotBool => write!(f, "Value is not a Bool"),
            ValueConversionError::NotInt => write!(f, "Value is not an Int"),
            ValueConversionError::NotBigInt => write!(f, "Value is not a BigInt"),
            ValueConversionError::NotFloat => write!(f, "Value is not a Float"),
            ValueConversionError::NotDouble => write!(f, "Value is not a Double"),
            ValueConversionError::NotText => write!(f, "Value is not Text"),
            ValueConversionError::NotByteArray => write!(f, "Value is not a ByteArray"),
            ValueConversionError::NotDate => write!(f, "Value is not a Date"),
            ValueConversionError::NotTime => write!(f, "Value is not a Time"),
            ValueConversionError::NotTimestamp => write!(f, "Value is not a Timestamp"),
            ValueConversionError::NotUuid => write!(f, "Value is not a Uuid"),
            ValueConversionError::NotJson => write!(f, "Value is not a json value"),
        }
    }
}

impl core::error::Error for ValueConversionError {}
