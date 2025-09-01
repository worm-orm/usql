use alloc::boxed::Box;
use bytes::Bytes;
use ordered_float::OrderedFloat;

use crate::{JsonValue, Value};
use core::fmt;

use super::Type;

macro_rules! impl_is {
    ($($name: ident => $variant: ident),+) => {
        $(
            pub fn $name(&self) -> bool {
                matches!(self, Self::$variant(_))
            }
        )*
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueRef<'a> {
    Null,
    Bool(bool),
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(OrderedFloat<f32>),
    Double(OrderedFloat<f64>),
    Text(&'a str),
    ByteArray(&'a [u8]),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Timestamp(chrono::NaiveDateTime),
    Uuid(uuid::Uuid),
    Json(&'a JsonValue),
    Array(&'a [Value]),
}

impl ValueRef<'_> {
    pub fn get_type(&self) -> Option<Type> {
        match self {
            Self::Null => None,
            Self::Bool(_) => Some(Type::Bool),
            Self::SmallInt(_) => Some(Type::SmallInt),
            Self::Int(_) => Some(Type::SmallInt),
            Self::BigInt(_) => Some(Type::BigInt),
            Self::Float(_) => Some(Type::Real),
            Self::Double(_) => Some(Type::Double),
            Self::Text(_) => Some(Type::Text),
            Self::ByteArray(_) => Some(Type::Blob),
            Self::Date(_) => Some(Type::Date),
            Self::Time(_) => Some(Type::Time),
            Self::Timestamp(_) => Some(Type::DateTime),
            Self::Uuid(_) => Some(Type::Uuid),
            Self::Json(_) => Some(Type::Json),
            Self::Array(values) => {
                if let Some(first) = values.first() {
                    Some(Type::Array(Box::new(first.get_type()?)))
                } else {
                    None
                }
            }
        }
    }

    impl_is!(
        is_bool => Bool,
        is_smallint => SmallInt,
        is_int => Int,
        is_bigint => BigInt,
        is_float => Float,
        is_double => Double,
        is_text => Text,
        is_binary => ByteArray,
        is_date => Date,
        is_datetime => Timestamp,
        is_time => Time,
        is_uuid => Uuid,
        is_json => Json,
        is_array => Array
    );

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Null => ValueRef::Null,
            Value::Bool(b) => ValueRef::Bool(*b),
            Value::SmallInt(i) => ValueRef::SmallInt(*i),
            Value::Int(i) => ValueRef::Int(*i),
            Value::BigInt(i) => ValueRef::BigInt(*i),
            Value::Float(f) => ValueRef::Float(*f),
            Value::Double(d) => ValueRef::Double(*d),
            Value::Text(s) => ValueRef::Text(s),
            Value::ByteArray(b) => ValueRef::ByteArray(b),
            Value::Date(d) => ValueRef::Date(*d),
            Value::Time(t) => ValueRef::Time(*t),
            Value::Timestamp(ts) => ValueRef::Timestamp(*ts),
            Value::Uuid(u) => ValueRef::Uuid(*u),
            Value::Json(j) => ValueRef::Json(j),
            Value::Array(arr) => ValueRef::Array(arr),
        }
    }
}

impl<'a> From<ValueRef<'a>> for Value {
    fn from(value_ref: ValueRef<'a>) -> Self {
        match value_ref {
            ValueRef::Null => Value::Null,
            ValueRef::Bool(b) => Value::Bool(b),
            ValueRef::SmallInt(i) => Value::SmallInt(i),
            ValueRef::Int(i) => Value::Int(i),
            ValueRef::BigInt(i) => Value::BigInt(i),
            ValueRef::Float(f) => Value::Float(f),
            ValueRef::Double(d) => Value::Double(d),
            ValueRef::Text(s) => Value::Text(s.into()),
            ValueRef::ByteArray(b) => Value::ByteArray(Bytes::copy_from_slice(b)),
            ValueRef::Date(d) => Value::Date(d),
            ValueRef::Time(t) => Value::Time(t),
            ValueRef::Timestamp(ts) => Value::Timestamp(ts),
            ValueRef::Uuid(u) => Value::Uuid(u),
            ValueRef::Json(j) => Value::Json(j.clone()),
            ValueRef::Array(arr) => Value::Array(arr.to_vec()),
        }
    }
}

impl fmt::Display for ValueRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueRef::Null => write!(f, "null"),
            ValueRef::Bool(b) => write!(f, "{}", b),
            ValueRef::SmallInt(i) => write!(f, "{}", i),
            ValueRef::Int(i) => write!(f, "{}", i),
            ValueRef::BigInt(i) => write!(f, "{}", i),
            ValueRef::Float(flt) => write!(f, "{}", flt),
            ValueRef::Double(dbl) => write!(f, "{}", dbl),
            ValueRef::Text(s) => write!(f, "\"{}\"", s),
            ValueRef::ByteArray(b) => write!(f, "{:?}", b),
            ValueRef::Date(d) => write!(f, "{}", d),
            ValueRef::Time(t) => write!(f, "{}", t),
            ValueRef::Timestamp(ts) => write!(f, "{}", ts),
            ValueRef::Uuid(u) => write!(f, "{}", u),
            ValueRef::Json(j) => write!(f, "{}", j),
            ValueRef::Array(arr) => {
                write!(f, "[")?;
                for (i, value) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ValueRef::from(value))?;
                }
                write!(f, "]")
            }
        }
    }
}
