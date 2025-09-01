use alloc::string::String;
use bytes::Bytes;

use crate::{Atom, JsonValue, Value};

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
            Ok(s.into())
        } else {
            Err(ValueConversionError::NotText)
        }
    }
}

impl TryFrom<Value> for Atom {
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
